"use client";

import React, { useState, useEffect, useCallback } from 'react';
import { 
  Upload, 
  Activity, 
  CheckCircle2, 
  AlertCircle, 
  Download, 
  Terminal, 
  Layers,
  Search,
  ArrowRight,
  Database
} from 'lucide-react';
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

const API_BASE = "/api";

interface JobStatus {
  job_id: string;
  status: string;
  total_rows: number;
  processed_rows: number;
}

interface HitlItem {
  hitl_id: number;
  raw_header: string;
  sample_values: string; // JSON string
}

const ESRS_TARGETS = [
  { label: "Scope 1: Natural Gas (Energy)", value: "E1_Scope1_StationaryCombustion_NaturalGas_Energy" },
  { label: "Scope 1: Natural Gas (Volume)", value: "E1_Scope1_StationaryCombustion_NaturalGas_Vol" },
  { label: "Scope 1: Diesel Fleet (US)", value: "E1_Scope1_MobileCombustion_Diesel_Fleet_US" },
  { label: "Scope 1: Diesel Fleet (UK)", value: "E1_Scope1_MobileCombustion_Diesel_Fleet_UK" },
  { label: "Scope 1: Gasoline Fleet (US)", value: "E1_Scope1_MobileCombustion_Gasoline_Fleet_US" },
  { label: "Scope 2: Electricity (US National)", value: "E1_Scope2_LocationBased_Electricity_US_National" },
  { label: "Scope 2: Electricity (UK Grid)", value: "E1_Scope2_LocationBased_Electricity_UK_Grid" },
  { label: "Scope 3: Water Supply (US)", value: "E1_Scope3_PurchasedGoods_WaterSupply_US" },
  { label: "Scope 3: Municipal Waste (US)", value: "E1_Scope3_WasteGenerated_MunicipalWaste_US" },
  { label: "Social: Workforce Metrics", value: "Social_Workforce_Metrics" },
];

export default function OperatorCockpit() {
  const [file, setFile] = useState<File | null>(null);
  const [jobId, setJobId] = useState<string | null>(null);
  const [status, setStatus] = useState<JobStatus | null>(null);
  const [hitlItems, setHitlItems] = useState<HitlItem[]>([]);
  const [logs, setLogs] = useState<string[]>(["[SYSTEM] Ready for operation..."]);
  const [isUploading, setIsUploading] = useState(false);
  const [resolvingId, setResolvingId] = useState<number | null>(null);

  const addLog = useCallback((msg: string) => {
    setLogs(prev => [`[${new Date().toLocaleTimeString()}] ${msg}`, ...prev].slice(0, 50));
  }, []);

  // Poll Job Status
  useEffect(() => {
    if (!jobId || (status?.status === 'COMPLETE' || status?.status === 'FAILED')) return;

    const interval = setInterval(async () => {
      try {
        const res = await fetch(`${API_BASE}/jobs/${jobId}/status`);
        const data = await res.json();
        
        if (data.status !== status?.status) {
          addLog(`Status changed: ${data.status}`);
        }
        setStatus(data);

        if (data.status === 'PAUSED_HITL') {
          fetchHitlItems();
        } else {
          setHitlItems([]);
        }
      } catch (err) {
        addLog(`Error polling status: ${err}`);
      }
    }, 2000);

    return () => clearInterval(interval);
  }, [jobId, status, addLog]);

  const fetchHitlItems = async () => {
    if (!jobId) return;
    try {
      const res = await fetch(`${API_BASE}/jobs/${jobId}/hitl`);
      const data = await res.json();
      setHitlItems(data.hitl_items || []);
    } catch (err) {
      addLog(`Error fetching HITL items: ${err}`);
    }
  };

  const handleUpload = async () => {
    if (!file) return;
    setIsUploading(true);
    addLog(`Initiating upload: ${file.name}`);

    const formData = new FormData();
    formData.append('file', file);

    try {
      const res = await fetch(`${API_BASE}/upload`, {
        method: 'POST',
        body: formData,
      });
      const data = await res.json();
      setJobId(data.job_id);
      addLog(`Upload successful. Job ID assigned: ${data.job_id}`);
    } catch (err) {
      addLog(`Upload failed: ${err}`);
    } finally {
      setIsUploading(false);
    }
  };

  const resolveHitl = async (hitlId: number, targetCat: string) => {
    if (!targetCat) return;
    setResolvingId(hitlId);
    addLog(`Resolving HITL ${hitlId} as ${targetCat}`);
    try {
      const res = await fetch(`${API_BASE}/hitl/resolve`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          hitl_id: hitlId,
          target_category: targetCat,
          industry: "General",
          expected_unit: "kWh",
          normalized_unit: "kWh",
          jurisdiction: "US"
        }),
      });
      const data = await res.json();
      if (data.status === 'RESOLVED') {
        addLog(`HITL Resolved. Resume signal sent.`);
        setHitlItems(prev => prev.filter(i => i.hitl_id !== hitlId));
      }
    } catch (err) {
      addLog(`Resolution failed: ${err}`);
    } finally {
      setResolvingId(null);
    }
  };

  const progress = status ? (status.total_rows > 0 ? (status.processed_rows / status.total_rows) * 100 : 0) : 0;

  return (
    <div className="min-h-screen bg-[#f5f5f7] text-[#1d1d1f] font-sans selection:bg-[#007aff]/20">
      {/* Header */}
      <header className="sticky top-0 z-50 bg-white/80 backdrop-blur-md border-b border-[#d2d2d7]">
        <div className="max-w-5xl mx-auto px-6 h-16 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Layers className="text-[#007aff] w-6 h-6" />
            <h1 className="text-[19px] font-semibold tracking-tight uppercase">
              Targoo V2 <span className="font-light text-[#86868b]">—— Data Refinery Operator</span>
            </h1>
          </div>
          <div className="flex items-center gap-4 text-[13px] font-medium">
            <span className={cn(
              "flex items-center gap-1.5 px-3 py-1 rounded-full",
              status?.status === 'PROCESSING' ? "bg-[#007aff]/10 text-[#007aff]" : 
              status?.status === 'PAUSED_HITL' ? "bg-orange-100 text-orange-600" :
              status?.status === 'COMPLETE' ? "bg-green-100 text-green-600" : "bg-[#86868b]/10 text-[#86868b]"
            )}>
              <Activity className="w-3.5 h-3.5" />
              {status?.status || "STANDBY"}
            </span>
          </div>
        </div>
      </header>

      <main className="max-w-5xl mx-auto px-6 py-12 space-y-8">
        {/* Ingest Zone */}
        <section className="bg-white rounded-[24px] p-8 border border-[#d2d2d7] shadow-sm transition-all hover:shadow-md">
          <div className="flex flex-col items-center justify-center border-2 border-dashed border-[#d2d2d7] rounded-[18px] py-16 px-4 bg-[#fbfbfd]">
            <Upload className="w-12 h-12 text-[#007aff] mb-4 opacity-80" />
            <h2 className="text-[24px] font-semibold mb-2">Ready to refine data?</h2>
            <p className="text-[#86868b] mb-8 text-center max-w-sm">
              Drag and drop your raw CSV or XLSX files here. The refinery will handle the physics.
            </p>
            
            <div className="flex flex-col items-center gap-4 w-full max-w-md">
              <input 
                type="file" 
                onChange={(e) => setFile(e.target.files?.[0] || null)}
                className="block w-full text-sm text-[#86868b] file:mr-4 file:py-2.5 file:px-6 file:rounded-full file:border-0 file:text-[14px] file:font-semibold file:bg-[#007aff] file:text-white hover:file:bg-[#0077ed] transition-all cursor-pointer"
              />
              
              <button 
                onClick={handleUpload}
                disabled={!file || isUploading}
                className="w-full bg-[#1d1d1f] text-white py-3.5 rounded-full font-semibold text-[17px] hover:scale-[1.02] active:scale-[0.98] disabled:opacity-50 disabled:hover:scale-100 transition-all flex items-center justify-center gap-2"
              >
                {isUploading ? "PROCESSOR INITIALIZING..." : "EL PSY KONGROO"}
                {!isUploading && <ArrowRight className="w-5 h-5" />}
              </button>
            </div>
          </div>
        </section>

        {/* Progress & Active Job */}
        {jobId && (
          <section className="grid md:grid-cols-3 gap-6 animate-in fade-in slide-in-from-bottom-4 duration-700">
            <div className="md:col-span-2 bg-white rounded-[24px] p-8 border border-[#d2d2d7] shadow-sm space-y-6">
              <div className="flex justify-between items-start">
                <div>
                  <label className="text-[13px] font-semibold text-[#86868b] uppercase tracking-wider">Active Job Reference</label>
                  <p className="text-[17px] font-mono font-bold mt-1">{jobId}</p>
                </div>
                <div className="text-right">
                  <label className="text-[13px] font-semibold text-[#86868b] uppercase tracking-wider">Indexed Content</label>
                  <p className="text-[17px] font-bold mt-1">{status?.processed_rows || 0} / {status?.total_rows || '...'}</p>
                </div>
              </div>

              <div className="space-y-2">
                <div className="h-3 bg-[#f5f5f7] rounded-full overflow-hidden border border-[#d2d2d7]/50">
                  <div 
                    className="h-full bg-[#007aff] transition-all duration-1000 ease-out"
                    style={{ width: `${progress}%` }}
                  />
                </div>
                <div className="flex justify-between text-[13px] font-medium text-[#86868b]">
                  <span>REFINERY PROGRESS</span>
                  <span>{Math.round(progress)}%</span>
                </div>
              </div>

              {status?.status === 'COMPLETE' && (
                <a 
                  href={`${API_BASE}/download/${jobId}`}
                  className="inline-flex items-center gap-2 bg-[#34c759] text-white px-8 py-3 rounded-full font-semibold hover:bg-[#30b753] transition-all shadow-sm shadow-[#34c759]/20"
                >
                  <Download className="w-5 h-5" />
                  DOWNLOAD $8,000 PACKAGE
                </a>
              )}
            </div>

            {/* Terminal Logs */}
            <div className="bg-[#1d1d1f] rounded-[24px] p-6 text-[#f5f5f7] font-mono text-[12px] flex flex-col shadow-xl">
              <div className="flex items-center gap-2 mb-4 border-b border-white/10 pb-2">
                <Terminal className="w-4 h-4 text-[#34c759]" />
                <span className="uppercase font-bold tracking-widest text-[#86868b]">System Console</span>
              </div>
              <div className="flex-1 overflow-y-auto space-y-1.5 opacity-90">
                {logs.map((log, i) => (
                  <div key={i} className="leading-tight break-all">
                    {log}
                  </div>
                ))}
              </div>
            </div>
          </section>
        )}

        {/* HITL Interface */}
        {status?.status === 'PAUSED_HITL' && hitlItems.length > 0 && (
          <div className="space-y-6 animate-in zoom-in-95 duration-500">
            <div className="flex items-center gap-3 px-2">
              <Database className="w-6 h-6 text-[#007aff]" />
              <h3 className="text-[20px] font-bold tracking-tight">Manual Data Engineering Panel</h3>
            </div>
            
            {hitlItems.map((item) => {
              const samples: string[] = JSON.parse(item.sample_values || "[]");
              return (
                <div key={item.hitl_id} className="bg-white rounded-[24px] border border-orange-200 overflow-hidden shadow-lg shadow-orange-100/50">
                  <div className="bg-orange-50 px-8 py-4 border-b border-orange-100">
                    <p className="text-orange-800 text-[12px] font-bold uppercase tracking-widest">Action Required: Unidentified Semantic Pattern</p>
                  </div>
                  
                  <div className="p-8 space-y-8">
                    <div className="grid md:grid-cols-2 gap-8">
                      <div className="space-y-4">
                        <div>
                          <p className="text-[13px] font-bold text-[#86868b] uppercase mb-1">Raw Header Name</p>
                          <h4 className="text-[22px] font-mono font-bold text-[#34c759]">{item.raw_header}</h4>
                        </div>
                        
                        <div className="bg-[#f5f5f7] p-4 rounded-xl border border-[#d2d2d7]/50">
                          <p className="text-[11px] font-bold text-[#86868b] uppercase mb-2 tracking-wider">Data Preview (First 5 Rows)</p>
                          <div className="flex flex-wrap gap-2">
                            {samples.map((val, idx) => (
                              <span key={idx} className="bg-white px-2 py-1 rounded border border-[#d2d2d7] font-mono text-[13px]">
                                {val}
                              </span>
                            ))}
                          </div>
                        </div>
                      </div>

                      <div className="space-y-6">
                        <div className="space-y-2">
                          <label className="text-[13px] font-bold text-[#1d1d1f] uppercase tracking-wide">Assign ESRS/GHG Target</label>
                          <div className="relative">
                            <select 
                              id={`cat-${item.hitl_id}`}
                              className="w-full pl-4 pr-10 py-3.5 rounded-xl bg-[#f5f5f7] border border-[#d2d2d7] focus:ring-2 focus:ring-[#007aff] outline-none appearance-none font-medium transition-all cursor-pointer"
                            >
                              <option value="">Select a verified category...</option>
                              {ESRS_TARGETS.map(t => (
                                <option key={t.value} value={t.value}>{t.label}</option>
                              ))}
                            </select>
                            <div className="absolute right-4 top-4 pointer-events-none">
                              <ArrowRight className="w-4 h-4 rotate-90 text-[#86868b]" />
                            </div>
                          </div>
                        </div>

                        <button 
                          onClick={() => {
                            const select = document.getElementById(`cat-${item.hitl_id}`) as HTMLSelectElement;
                            resolveHitl(item.hitl_id, select.value);
                          }}
                          disabled={resolvingId === item.hitl_id}
                          className="w-full bg-[#007aff] text-white py-4 rounded-xl font-bold hover:bg-[#0077ed] hover:scale-[1.01] active:scale-[0.99] transition-all flex items-center justify-center gap-2 shadow-md shadow-[#007aff]/20"
                        >
                          {resolvingId === item.hitl_id ? "PROCESSING..." : "VERIFY & LEARN"}
                          <CheckCircle2 className="w-5 h-5" />
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </main>

      {/* Footer Disclaimer */}
      <footer className="max-w-5xl mx-auto px-6 py-12 border-t border-[#d2d2d7] text-[12px] text-[#86868b] space-y-4">
        <p className="font-bold">VERIFICATION PROTOCOL v2.0.0</p>
        <p>
          This interface is for authorized data engineers only. All interactions are recorded in the 
          WORM-protected ledger. Determination of emission factors and category mappings must adhere 
          to the GHG Protocol Corporate Standard and relevant jurisdictional regulations (SB 253 / SECR).
        </p>
        <p>© 2026 Targoo — Zero-Trust ESG Data Refinery.</p>
      </footer>
    </div>
  );
}
