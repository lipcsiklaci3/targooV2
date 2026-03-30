"use client";

import React, { useState } from "react";
import DropZone from "./components/DropZone";
import ProcessingStatus from "./components/ProcessingStatus";
import ResultCard, { JobSummary } from "./components/ResultCard";
import DownloadButton from "./components/DownloadButton";
import { AlertCircle, RefreshCcw } from "lucide-react";

type AppState = "IDLE" | "UPLOADING" | "PROCESSING" | "COMPLETE" | "ERROR";

export default function Home() {
  const [state, setState] = useState<AppState>("IDLE");
  const [files, setFiles] = useState<File[]>([]);
  const [summary, setSummary] = useState<JobSummary | null>(null);
  const [zipBlob, setZipBlob] = useState<Blob | null>(null);
  const [error, setError] = useState<string | null>(null);

  const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8000";

  const handleUpload = async () => {
    if (files.length === 0) return;

    setState("UPLOADING");
    setError(null);

    const formData = new FormData();
    files.forEach((file) => formData.append("files", file));

    try {
      const response = await fetch(`${API_URL}/jobs/upload`, {
        method: "POST",
        body: formData,
      });

      if (!response.ok) {
        const errText = await response.text();
        throw new Error(errText || `Server responded with ${response.status}`);
      }

      setState("PROCESSING");

      // Extract summary from header
      const summaryHeader = response.headers.get("X-Job-Summary");
      if (summaryHeader) {
        try {
          setSummary(JSON.parse(summaryHeader));
        } catch (e) {
          console.error("Failed to parse X-Job-Summary header", e);
        }
      }

      const blob = await response.blob();
      setZipBlob(blob);
      
      // Artificial delay for that "Magic" feel
      setTimeout(() => {
        setState("COMPLETE");
      }, 3000);

    } catch (err: any) {
      console.error(err);
      setError(err.message === "Failed to fetch" ? "Connection failed. Is the backend running?" : err.message);
      setState("ERROR");
    }
  };

  const reset = () => {
    setFiles([]);
    setSummary(null);
    setZipBlob(null);
    setError(null);
    setState("IDLE");
  };

  return (
    <main className="flex flex-col items-center min-h-dvh bg-background overflow-x-hidden pt-12">
      {/* Header */}
      <div className="flex flex-col items-center gap-1 mb-8">
        <h1 className="text-3xl font-black tracking-tighter text-white">
          TARGOO <span className="text-primary">V2</span>
        </h1>
        <div className="h-px w-8 bg-primary/40 mb-1"></div>
        <p className="text-[10px] uppercase tracking-[0.3em] text-zinc-500 font-bold">
          ESG Data Refinery
        </p>
      </div>

      {/* Main Content */}
      <div className="flex-1 w-full flex flex-col items-center justify-center">
        {state === "IDLE" && (
          <DropZone 
            onFilesSelected={setFiles} 
            files={files} 
            onTrigger={handleUpload} 
          />
        )}

        {(state === "UPLOADING" || state === "PROCESSING") && (
          <ProcessingStatus />
        )}

        {state === "COMPLETE" && summary && (
          <div className="flex flex-col items-center gap-6 w-full">
            <ResultCard summary={summary} />
            {zipBlob && <DownloadButton zipBlob={zipBlob} jobId={summary.job_id} />}
            <button 
                onClick={reset}
                className="flex items-center gap-2 text-zinc-500 text-xs font-bold uppercase tracking-widest hover:text-white transition-colors py-4"
            >
                <RefreshCcw size={14} />
                Refine More Data
            </button>
          </div>
        )}

        {state === "ERROR" && (
          <div className="px-6 w-full max-w-[390px] animate-in fade-in zoom-in-95">
            <div className="bg-red-500/10 border border-red-500/20 rounded-3xl p-8 flex flex-col items-center text-center gap-4">
              <AlertCircle size={48} className="text-red-500" />
              <div className="space-y-1">
                  <h3 className="text-lg font-bold text-white">Refinery Failure</h3>
                  <p className="text-zinc-400 text-xs leading-relaxed">{error}</p>
              </div>
              <button 
                onClick={reset}
                className="mt-4 px-6 py-3 bg-zinc-900 rounded-xl text-xs font-bold uppercase tracking-widest text-white active:scale-95 transition-all"
              >
                Try Again
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Footer */}
      <footer className="w-full py-8 flex flex-col items-center gap-2">
        <p className="text-[9px] text-zinc-700 uppercase tracking-widest font-bold">
          Powered by ESRS • CSRD • SFDR
        </p>
        <div className="flex gap-4 opacity-20 grayscale">
            <div className="text-[8px] border border-white px-1 rounded">EU</div>
            <div className="text-[8px] border border-white px-1 rounded">UN</div>
            <div className="text-[8px] border border-white px-1 rounded">GRI</div>
        </div>
      </footer>
    </main>
  );
}
