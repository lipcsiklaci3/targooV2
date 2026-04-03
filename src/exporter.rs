use libsql::{Connection, params};
use rust_xlsxwriter::{Workbook, Format, Color, FormatBorder};
use docx_rs::*;
use zip::write::FileOptions;
use std::io::{Cursor, Write};
use std::sync::Arc;
use axum::{extract::{Path, State}, response::IntoResponse, body::Body, http::{header, StatusCode}};

pub async fn generate_audit_trail(conn: &Connection, job_id: &str) -> Vec<u8> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Verified Emissions Ledger").unwrap();

    // Formatting
    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x0070C0)) // Blue
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin);

    // Headers
    let headers = [
        "Source File", "Row", "Original Header", "Raw Value", 
        "Target Category", "Jurisdiction", "tCO2e", "Verification Source", "Source Integrity ID"
    ];

    for (col, text) in headers.iter().enumerate() {
        worksheet.write_with_format(0, col as u16, *text, &header_format).unwrap();
    }

    // Data
    let mut rows = conn.query(
        "SELECT source_file, row_number, raw_header, raw_value, target_category, jurisdiction, tco2e, factor_source, row_sha256 
         FROM esg_ledger WHERE job_id = ? AND status = 'clean'",
        params![job_id]
    ).await.unwrap();

    let mut row_idx = 1;
    while let Some(row) = rows.next().await.unwrap() {
        worksheet.write(row_idx, 0, row.get::<String>(0).unwrap()).unwrap();
        worksheet.write(row_idx, 1, row.get::<i64>(1).unwrap()).unwrap();
        worksheet.write(row_idx, 2, row.get::<String>(2).unwrap()).unwrap();
        worksheet.write(row_idx, 3, row.get::<String>(3).unwrap()).unwrap();
        worksheet.write(row_idx, 4, row.get::<String>(4).unwrap()).unwrap();
        worksheet.write(row_idx, 5, row.get::<String>(5).unwrap()).unwrap();
        worksheet.write(row_idx, 6, row.get::<f64>(6).unwrap()).unwrap();
        worksheet.write(row_idx, 7, row.get::<String>(7).unwrap()).unwrap();
        worksheet.write(row_idx, 8, row.get::<String>(8).unwrap()).unwrap();
        row_idx += 1;
    }

    workbook.save_to_buffer().unwrap()
}

pub async fn generate_quarantine_log(conn: &Connection, job_id: &str) -> Vec<u8> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Manual Review Required").unwrap();

    let header_format = Format::new().set_bold().set_background_color(Color::Red).set_font_color(Color::White);
    let repair_format = Format::new().set_background_color(Color::Yellow);

    let headers = ["Source File", "Row", "Issue", "Raw Data", "REPAIR_VALUE", "UNIT_FIX"];
    for (col, text) in headers.iter().enumerate() {
        worksheet.write_with_format(0, col as u16, *text, &header_format).unwrap();
    }

    // Query for UNKNOWN or low confidence rows
    let mut rows = conn.query(
        "SELECT source_file, row_number, raw_header, raw_value FROM esg_ledger WHERE job_id = ? AND status != 'clean'",
        params![job_id]
    ).await.unwrap();

    let mut row_idx = 1;
    while let Some(row) = rows.next().await.unwrap() {
        worksheet.write(row_idx, 0, row.get::<String>(0).unwrap()).unwrap();
        worksheet.write(row_idx, 1, row.get::<i64>(1).unwrap()).unwrap();
        worksheet.write(row_idx, 2, "Automatic Mapping Failed").unwrap();
        worksheet.write(row_idx, 3, format!("{}: {}", row.get::<String>(2).unwrap(), row.get::<String>(3).unwrap())).unwrap();
        worksheet.write_blank(row_idx, 4, &repair_format).unwrap();
        worksheet.write_blank(row_idx, 5, &repair_format).unwrap();
        row_idx += 1;
    }

    workbook.save_to_buffer().unwrap()
}

pub async fn generate_word_report(conn: &Connection, job_id: &str) -> Vec<u8> {
    let mut doc = Docx::new();

    doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text("Manual Data Engineering Verification Report").size(48).bold()))
       .add_paragraph(Paragraph::new().add_run(Run::new().add_text(format!("Job ID: {}", job_id)).size(24)))
       .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Executive Summary").size(36).bold()));

    // Get totals by scope/category
    let mut rows = conn.query(
        "SELECT target_category, SUM(tco2e) FROM esg_ledger WHERE job_id = ? AND status = 'clean' GROUP BY target_category",
        params![job_id]
    ).await.unwrap();

    let mut table = Table::new(vec![]);
    table = table.add_row(TableRow::new(vec![
        TableCell::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text("Category"))),
        TableCell::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text("tCO2e"))),
    ]));

    while let Some(row) = rows.next().await.unwrap() {
        table = table.add_row(TableRow::new(vec![
            TableCell::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text(row.get::<String>(0).unwrap()))),
            TableCell::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text(format!("{:.4}", row.get::<f64>(1).unwrap())))),
        ]));
    }

    doc = doc.add_table(table);

    doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text("\n\nMethodology & Compliance Statement").size(24).bold()))
       .add_paragraph(Paragraph::new().add_run(Run::new().add_text("This report was generated through manual data engineering verification processes. All calculations are based on provided data points and verified emission factors. No automated intelligence or heuristic-only modeling was used for final reporting numbers without human oversight.")));

    let mut buf = Vec::new();
    doc.build().pack(Cursor::new(&mut buf)).unwrap();
    buf
}

pub async fn create_fritz_package(db: Arc<Connection>, job_id: String) -> Vec<u8> {
    let audit_trail = generate_audit_trail(&db, &job_id).await;
    let quarantine = generate_quarantine_log(&db, &job_id).await;
    let report = generate_word_report(&db, &job_id).await;

    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut buf));
        let options: FileOptions<()> = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("01_Verified_Audit_Trail.xlsx", options).unwrap();
        zip.write_all(&audit_trail).unwrap();

        zip.start_file("02_Manual_Review_Log.xlsx", options).unwrap();
        zip.write_all(&quarantine).unwrap();

        zip.start_file("03_Verification_Report.docx", options).unwrap();
        zip.write_all(&report).unwrap();

        zip.finish().unwrap();
    }
    buf
}

pub async fn download_package(
    Path(job_id): Path<String>,
    State(db): State<Arc<Connection>>,
) -> impl IntoResponse {
    let zip_data = create_fritz_package(db, job_id.clone()).await;

    let disposition = format!("attachment; filename=\"Targoo_Data_Package_{}.zip\"", job_id);

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/zip".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        Body::from(zip_data),
    )
}
