"use client";

import React from "react";
import { CheckCircle2, AlertTriangle, Database, ArrowRight, Zap, Target } from "lucide-react";

export interface JobSummary {
  job_id: string;
  total_rows: number;
  total_tco2e: number;
  clean_rows: number;
  best_effort_rows: number;
  quarantined_rows: number;
}

interface ResultCardProps {
  summary: JobSummary;
}

export default function ResultCard({ summary }: ResultCardProps) {
  return (
    <div className="w-full max-w-[390px] px-6 space-y-6 animate-in zoom-in-95 duration-500">
      <div className="bg-[#111] border border-zinc-800 rounded-3xl p-6 relative overflow-hidden">
        <div className="absolute top-0 right-0 p-4 opacity-5">
            <Zap size={120} fill="white" />
        </div>

        <div className="flex flex-col items-center text-center mb-6">
          <div className="bg-green-500/10 p-3 rounded-full mb-4">
            <CheckCircle2 size={48} className="text-green-500" />
          </div>
          <h2 className="text-2xl font-bold text-white">Fritz Package Ready!</h2>
          <p className="text-zinc-500 text-xs mt-1">Refinery Run Complete</p>
        </div>

        <div className="grid grid-cols-2 gap-3 mb-6">
          <StatBox icon={<Database size={14}/>} label="Total Rows" value={summary.total_rows} />
          <StatBox icon={<CheckCircle2 size={14}/>} label="Clean" value={summary.clean_rows} color="text-green-500" />
          <StatBox icon={<AlertTriangle size={14}/>} label="Best Effort" value={summary.best_effort_rows} color="text-yellow-500" />
          <StatBox icon={<AlertTriangle size={14}/>} label="Quarantined" value={summary.quarantined_rows} color="text-red-500" />
        </div>

        <div className="bg-zinc-900/50 rounded-2xl p-5 border border-zinc-800 text-center">
            <div className="text-zinc-500 text-[10px] uppercase font-bold tracking-tighter mb-1">Total Carbon Footprint</div>
            <div className="text-3xl font-black text-white">
                {summary.total_tco2e.toFixed(3)} <span className="text-primary">tCO2e</span>
            </div>
        </div>
      </div>

      {summary.quarantined_rows > 0 && (
        <div className="bg-yellow-500/10 border border-yellow-500/20 rounded-2xl p-4 flex gap-3 items-start animate-in slide-in-from-top-2">
          <AlertTriangle className="text-yellow-500 shrink-0 mt-0.5" size={18} />
          <p className="text-yellow-200/80 text-xs leading-relaxed">
            <span className="font-bold text-yellow-500">{summary.quarantined_rows} rows</span> need your attention. Check the <code className="bg-black/40 px-1 rounded text-white">00_ACTION_REQUIRED</code> file inside your ZIP.
          </p>
        </div>
      )}
    </div>
  );
}

function StatBox({ icon, label, value, color = "text-zinc-400" }: { icon: React.ReactNode, label: string, value: number, color?: string }) {
  return (
    <div className="bg-zinc-900/40 border border-zinc-800/50 rounded-xl p-3">
      <div className="flex items-center gap-1.5 text-[10px] text-zinc-500 uppercase font-bold mb-1">
        {icon}
        {label}
      </div>
      <div className={`text-lg font-mono font-bold ${color}`}>
        {value}
      </div>
    </div>
  );
}
