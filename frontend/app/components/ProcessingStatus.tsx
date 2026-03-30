"use client";

import React, { useEffect, useState } from "react";

const MESSAGES = [
  "📥 Receiving files...",
  "🔍 Detecting file types...",
  "⚙️ Running Unit-Validator...",
  "🧮 Calculating tCO2e...",
  "📦 Assembling Fritz Package...",
  "✅ Almost done...",
];

export default function ProcessingStatus() {
  const [msgIndex, setMsgIndex] = useState(0);

  useEffect(() => {
    const interval = setInterval(() => {
      setMsgIndex((prev) => (prev + 1) % MESSAGES.length);
    }, 2000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="flex flex-col items-center justify-center gap-8 py-12 px-6">
      <div className="relative w-24 h-24">
        {/* Outer slow spin */}
        <div className="absolute inset-0 border-4 border-zinc-900 rounded-full"></div>
        <div className="absolute inset-0 border-4 border-t-primary rounded-full animate-spin"></div>
        {/* Inner fast spin */}
        <div className="absolute inset-4 border-2 border-zinc-900 rounded-full"></div>
        <div className="absolute inset-4 border-2 border-b-primary rounded-full animate-spin-slow"></div>
      </div>

      <div className="flex flex-col items-center gap-2">
        <h3 className="text-lg font-medium text-zinc-200 animate-pulse text-center min-h-[1.75rem]">
          {MESSAGES[msgIndex]}
        </h3>
        <p className="text-zinc-600 text-xs italic">El Psy Kongroo...</p>
      </div>

      <div className="w-full max-w-[280px] h-1.5 bg-zinc-900 rounded-full overflow-hidden relative">
        <div className="absolute inset-0 bg-primary/20"></div>
        <div 
          className="absolute inset-y-0 bg-primary w-1/3 rounded-full animate-[progress_2s_infinite_linear]"
          style={{
            animation: "progress-slide 1.5s infinite linear"
          }}
        ></div>
      </div>

      <style jsx>{`
        @keyframes progress-slide {
          0% { transform: translateX(-100%); }
          100% { transform: translateX(300%); }
        }
      `}</style>
    </div>
  );
}
