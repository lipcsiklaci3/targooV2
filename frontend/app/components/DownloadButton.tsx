"use client";

import React from "react";
import { Download } from "lucide-react";

interface DownloadButtonProps {
  zipBlob: Blob;
  jobId: string;
}

export default function DownloadButton({ zipBlob, jobId }: DownloadButtonProps) {
  const handleDownload = () => {
    const url = window.URL.createObjectURL(zipBlob);
    const a = document.createElement("a");
    const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
    a.href = url;
    a.download = `Fritz_Package_${jobId}_${timestamp}.zip`;
    document.body.appendChild(a);
    a.click();
    window.URL.revokeObjectURL(url);
    document.body.removeChild(a);
  };

  return (
    <div className="w-full max-w-[390px] px-6 pb-12 mt-auto">
      <button
        onClick={handleDownload}
        className="w-full bg-[#00b4b4] hover:bg-[#008f8f] text-white font-bold py-5 rounded-2xl flex flex-col items-center justify-center gap-1 active:scale-95 transition-all shadow-xl shadow-teal-900/30 ring-2 ring-teal-500/20"
      >
        <div className="flex items-center gap-2">
            <Download size={22} strokeWidth={3} />
            <span className="text-lg uppercase tracking-tight">Download Fritz Package</span>
        </div>
        <span className="text-[10px] opacity-70 font-medium">5 audit-ready files inside • application/zip</span>
      </button>
      <p className="text-center text-zinc-700 text-[10px] mt-4 uppercase tracking-[0.2em] font-medium">
        Valid for CSRD / ESRS / SFDR Reporting
      </p>
    </div>
  );
}
