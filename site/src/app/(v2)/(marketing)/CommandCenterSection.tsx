"use client";

import { useState } from "react";
import { Icon, faCheck } from "@rivet-gg/icons";

// Command Center section
export const CommandCenterSection = () => {
  const [activeTab, setActiveTab] = useState("monitoring");
  
  const tabs = [
    { id: "monitoring", title: "Monitoring", },
    { id: "logs", title: "Logs", },
    { id: "local-dev", title: "Local Dev", },
    { id: "collaboration", title: "Collaboration", },
  ];

  const features = [
    "Live Logs", "Crash Reporting", "Log Retention", "Error Tracing", "Performance Metrics", "Alerts"
  ];

  return (
    <div className="mx-auto max-w-7xl px-6 py-28 lg:py-44 lg:px-8 mt-16">
      <div className="text-center mb-12">
        <h2 className="text-3xl font-bold tracking-tight text-white">The command center your backend is missing</h2>
        <p className="mt-4 text-lg text-white/70">Complete visibility and control over your serverless infrastructure</p>
      </div>
      
      <div className="flex justify-center mb-12">
        <div className="inline-flex space-x-1">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              className={`px-6 py-3 rounded-md text-base font-medium transition-all duration-200 border ${
                activeTab === tab.id 
                  ? "bg-zinc-900 text-white border-zinc-700" 
                  : "text-white/60 hover:text-white hover:bg-black/10 border-transparent"
              }`}
              onClick={() => setActiveTab(tab.id)}
            >
              {tab.title}
            </button>
          ))}
        </div>
      </div>
      
      <div className="flex justify-center mb-12">
        <div className="flex space-x-12">
          {features.map((feature, index) => (
            <div key={index} className="flex items-center">
              <div className="text-white/40 mr-2">
                <Icon icon={faCheck} className="text-sm" />
              </div>
              <span className="text-sm text-white/80">{feature}</span>
            </div>
          ))}
        </div>
      </div>
      
      <div className="flex justify-center">
        <div className="w-full max-w-4xl h-[480px] bg-zinc-900 border border-white/5 rounded-xl overflow-hidden">
          {/* Placeholder for hub screenshot */}
          <div className="w-full h-full flex items-center justify-center text-white/40">
            Hub Screenshot Placeholder
          </div>
        </div>
      </div>
    </div>
  );
};