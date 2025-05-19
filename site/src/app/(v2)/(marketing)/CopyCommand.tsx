"use client";

import { Icon, faCopy, faCheck } from "@rivet-gg/icons";
import { useState } from "react";

// Copy command component with clipboard functionality
export const CopyCommand = ({ command }: { command: string }) => {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    // Copy the command without the $ symbol
    navigator.clipboard.writeText(command);
    setCopied(true);
    setTimeout(() => setCopied(false), 1000);
  };

  return (
    <div 
      onClick={handleCopy}
      className="inline-flex items-center bg-black/40 rounded-md border border-white/10 px-4 py-2.5 font-mono text-sm group relative cursor-pointer active:scale-[0.98] active:bg-black/60 transition-all"
    >
      <span className="text-gray-500 mr-2 font-mono">$</span>
      <span className="text-white/90 font-mono">{command}</span>
      <div
        className="ml-3 text-gray-400 group-hover:text-[#FF5C00] transition-colors"
        aria-label={copied ? "Copied!" : "Copy to clipboard"}
      >
        <Icon icon={copied ? faCheck : faCopy} className={`${copied ? "text-[#FF5C00]" : ""} transition-all`} />
      </div>
      <div className="absolute inset-0 bg-white/5 opacity-0 group-hover:opacity-100 group-active:opacity-0 transition-opacity rounded-md pointer-events-none"></div>
    </div>
  );
};