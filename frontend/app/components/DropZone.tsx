"use client";

import React, { useRef, useState } from "react";
import { Upload, FileText, Zap } from "lucide-react";

interface DropZoneProps {
  onFilesSelected: (files: File[]) => void;
  files: File[];
  onTrigger: () => void;
}

export default function DropZone({ onFilesSelected, files, onTrigger }: DropZoneProps) {
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files) {
      onFilesSelected(Array.from(e.target.files));
    }
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    if (e.dataTransfer.files) {
      onFilesSelected(Array.from(e.dataTransfer.files));
    }
  };

  return (
    <div className="w-full max-w-[390px] px-6 animate-in fade-in slide-in-from-bottom-4 duration-700">
      <div
        onClick={() => fileInputRef.current?.click()}
        onDragOver={(e) => e.preventDefault()}
        onDrop={handleDrop}
        className="dashed-border min-h-[320px] bg-[#111] flex flex-col items-center justify-center cursor-pointer active:scale-[0.98] transition-all p-6 text-center"
      >
        <input
          type="file"
          ref={fileInputRef}
          onChange={handleFileChange}
          multiple
          className="hidden"
          accept=".csv,.xlsx,.json,.xml"
        />
        
        <div className="text-6xl mb-4">🗑️</div>
        <h2 className="text-xl font-semibold mb-2">Drop your data files here</h2>
        <p className="text-zinc-500 text-sm mb-6">
          CSV, XLSX, JSON, XML supported
        </p>

        {files.length > 0 && (
          <div className="w-full space-y-2 mt-2">
            <div className="text-primary text-sm font-medium">
              {files.length} file(s) selected
            </div>
            <div className="max-h-32 overflow-y-auto space-y-2 py-2">
              {files.map((file, i) => (
                <div key={i} className="flex items-center gap-2 text-xs bg-zinc-900 p-2 rounded border border-zinc-800 text-zinc-300">
                  <FileText size={14} className="text-primary shrink-0" />
                  <span className="truncate">{file.name}</span>
                  <span className="ml-auto text-zinc-600">{(file.size / 1024).toFixed(0)}KB</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      {files.length > 0 && (
        <button
          onClick={(e) => {
            e.stopPropagation();
            onTrigger();
          }}
          className="w-full mt-6 bg-[#00b4b4] hover:bg-[#008f8f] text-white font-bold py-4 rounded-xl flex items-center justify-center gap-2 active:scale-95 transition-all shadow-lg shadow-teal-900/20"
        >
          <Zap fill="white" size={20} />
          El Psy Kongroo
        </button>
      )}
      
      {files.length === 0 && (
        <p className="text-center text-zinc-600 text-[10px] mt-4 uppercase tracking-widest font-bold">
          Tap to select files from device
        </p>
      )}
    </div>
  );
}
