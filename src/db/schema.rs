pub const CREATE_JOBS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'queued',
    total_files INTEGER DEFAULT 0,
    total_rows INTEGER DEFAULT 0,
    total_tco2e REAL DEFAULT 0.0,
    clean_rows INTEGER DEFAULT 0,
    best_effort_rows INTEGER DEFAULT 0,
    quarantined_rows INTEGER DEFAULT 0,
    output_zip_url TEXT,
    created_at TEXT NOT NULL,
    completed_at TEXT
);";

pub const CREATE_ESG_LEDGER_TABLE: &str = "
CREATE TABLE IF NOT EXISTS esg_ledger (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL,
    esrs_target TEXT NOT NULL,
    raw_header TEXT NOT NULL,
    raw_value TEXT NOT NULL,
    canonical_value REAL,
    canonical_unit TEXT NOT NULL,
    tco2e REAL,
    emission_factor REAL,
    factor_source TEXT,
    confidence REAL NOT NULL,
    status TEXT NOT NULL,
    warning TEXT,
    error TEXT,
    processed_at TEXT NOT NULL
);";

pub const CREATE_QUARANTINE_TABLE: &str = "
CREATE TABLE IF NOT EXISTS quarantine_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL,
    esrs_target TEXT,
    raw_header TEXT NOT NULL,
    raw_value TEXT NOT NULL,
    error TEXT NOT NULL,
    confidence REAL NOT NULL,
    action_required TEXT NOT NULL DEFAULT 'Please verify this data point and resubmit',
    created_at TEXT NOT NULL
);";

pub const CREATE_MAPPING_DICTIONARY_TABLE: &str = "
CREATE TABLE IF NOT EXISTS mapping_dictionary (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    raw_text TEXT NOT NULL UNIQUE,
    raw_text_normalized TEXT NOT NULL,
    esrs_target TEXT NOT NULL,
    canonical_unit TEXT NOT NULL,
    conversion_factor REAL NOT NULL DEFAULT 1.0,
    confidence REAL NOT NULL,
    match_source TEXT NOT NULL DEFAULT 'dictionary',
    times_used INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);";
