"use client";

import React, { useState } from 'react';
import { 
  Terminal, 
  Zap, 
  Globe, 
  ArrowRight, 
  Box,
  Check,
  Server,
  Cloud,
  Activity,
  CreditCard,
  Command,
  Network,
  HardDrive,
  Laptop
} from 'lucide-react';

// --- Shared Design Components ---

const Badge = ({ text, color = "orange" }) => {
  const colorClasses = {
    orange: "text-[#FF4500] border-[#FF4500]/20 bg-[#FF4500]/10",
    blue: "text-blue-400 border-blue-500/20 bg-blue-500/10",
    red: "text-red-400 border-red-500/20 bg-red-500/10",
    zinc: "text-zinc-400 border-zinc-500/20 bg-zinc-500/10",
  };
  
  return (
    <div className={`inline-flex items-center gap-2 px-3 py-1 rounded-full border text-xs font-medium mb-8 transition-colors cursor-default ${colorClasses[color]}`}>
      <span className={`w-1.5 h-1.5 rounded-full ${color === "orange" ? "bg-[#FF4500]" : color === "blue" ? "bg-blue-400" : color === "red" ? "bg-red-400" : "bg-zinc-400"} animate-pulse`} />
      {text}
    </div>
  );
};

const CodeBlock = ({ code, fileName = 'rivet.json' }) => {
  return (
    <div className="relative group rounded-xl overflow-hidden border border-white/10 bg-zinc-900/50 shadow-2xl backdrop-blur-xl">
      <div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-white/30 to-transparent z-10" />
      <div className="flex items-center px-4 py-3 border-b border-white/5 bg-white/5">
        <div className="flex items-center gap-2 mr-auto">
          <div className="w-3 h-3 rounded-full bg-red-500/20 border border-red-500/50" />
          <div className="w-3 h-3 rounded-full bg-yellow-500/20 border border-yellow-500/50" />
          <div className="w-3 h-3 rounded-full bg-green-500/20 border border-green-500/50" />
        </div>
        <div className="text-xs text-zinc-500 font-mono absolute left-1/2 -translate-x-1/2">
            {fileName}
        </div>
      </div>
      <div className="p-4 overflow-x-auto scrollbar-hide">
        <pre className="text-sm font-mono leading-relaxed text-zinc-300">
          <code>{code}</code>
        </pre>
      </div>
    </div>
  );
};

const FeatureCard = ({ title, description, icon: Icon, color = "orange" }) => {
  const getColorClasses = (col) => {
    switch (col) {
      case "orange":
        return {
          iconBg: "bg-[#FF4500]/10 text-[#FF4500] group-hover:bg-[#FF4500]/20",
          glow: "rgba(255, 69, 0, 0.15)",
          borderColor: "border-[#FF4500]",
          iconShadow: "group-hover:shadow-[0_0_15px_rgba(255,69,0,0.5)]"
        };
      case "blue":
        return {
          iconBg: "bg-blue-500/10 text-blue-400 group-hover:bg-blue-500/20",
          glow: "rgba(59, 130, 246, 0.15)",
          borderColor: "border-blue-500",
          iconShadow: "group-hover:shadow-[0_0_15px_rgba(59,130,246,0.5)]"
        };
      case "red":
        return {
          iconBg: "bg-red-500/10 text-red-400 group-hover:bg-red-500/20",
          glow: "rgba(239, 68, 68, 0.15)",
          borderColor: "border-red-500",
          iconShadow: "group-hover:shadow-[0_0_15px_rgba(239,68,68,0.5)]"
        };
      default:
        return {
          iconBg: "bg-zinc-500/10 text-zinc-400 group-hover:bg-zinc-500/20",
          glow: "rgba(161, 161, 170, 0.15)",
          borderColor: "border-zinc-500",
          iconShadow: "group-hover:shadow-[0_0_15px_rgba(161,161,170,0.5)]"
        };
    }
  };

  const colors = getColorClasses(color);

  return (
    <div className="group relative overflow-hidden rounded-2xl border border-white/5 bg-black/50 backdrop-blur-sm flex flex-col h-full p-6 transition-all duration-500">
       {/* Top Shine Highlight */}
       <div className="absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent" />
       
       {/* Top Left Reflection/Glow */}
       <div 
         className="pointer-events-none absolute inset-0 opacity-0 transition-opacity duration-500 group-hover:opacity-100"
         style={{
           background: `radial-gradient(circle at top left, ${colors.glow} 0%, transparent 50%)`
         }}
       />
       
       {/* Sharp Edge Highlight (Masked to Fade) */}
       <div className={`pointer-events-none absolute left-0 top-0 z-20 h-24 w-24 rounded-tl-2xl border-l border-t ${colors.borderColor} opacity-0 transition-opacity duration-500 [mask-image:linear-gradient(135deg,black_0%,transparent_50%)] group-hover:opacity-100`} />

       <div className="flex items-center gap-3 mb-4 relative z-10">
          <div className={`p-2 rounded ${colors.iconBg} transition-all duration-500 ${colors.iconShadow}`}>
            <Icon className="w-5 h-5" />
          </div>
          <h3 className="text-lg font-medium text-white tracking-tight">{title}</h3>
       </div>
       <p className="text-sm text-zinc-400 leading-relaxed relative z-10 flex-grow">
         {description}
       </p>
    </div>
  );
};

// --- Page Sections ---

const Hero = () => (
  <section className="relative pt-32 pb-20 md:pt-48 md:pb-32 overflow-hidden">
    <div className="absolute top-0 left-1/2 -translate-x-1/2 w-[1000px] h-[500px] bg-white/[0.02] blur-[100px] rounded-full pointer-events-none" />
    
    <div className="max-w-7xl mx-auto px-6 relative z-10">
      <div className="flex flex-col items-center text-center">
        <div className="max-w-3xl mb-16">
          <h1 className="text-5xl md:text-7xl font-medium text-white tracking-tighter leading-[1.1] mb-6">
            Rivet Cloud
          </h1>
          
          <p className="text-lg md:text-xl text-zinc-400 leading-relaxed mb-8 max-w-2xl mx-auto">
             Rivet sits between your clients and your infrastructure. We manage the persistent connections, global routing, and actor state orchestrating your logic wherever it runs.
          </p>

          <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
            <a href="https://dashboard.rivet.dev"
              className="font-v2 inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black subpixel-antialiased shadow-sm transition-colors hover:bg-zinc-200 w-full sm:w-auto"
            >
              Get Started for Free
              <ArrowRight className="w-4 h-4" />
            </a>
            <button 
              onClick={() => document.getElementById('pricing')?.scrollIntoView({ behavior: 'smooth' })}
              className="font-v2 inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white subpixel-antialiased shadow-sm transition-colors hover:border-white/20 w-full sm:w-auto"
            >
              <CreditCard className="w-4 h-4" />
              See Pricing
            </button>
          </div>
        </div>

        {/* Main Diagram Container */}
        <div className="flex flex-col lg:flex-row items-center justify-center w-full">
             
             {/* 1. Source: Clients */}
             <div className="w-full max-w-[280px] p-6 rounded-2xl bg-zinc-900/50 border border-white/10 flex flex-col items-center text-center z-20 relative group hover:border-blue-500/30 transition-colors">
                 <div className="w-16 h-16 rounded-full bg-blue-500/10 flex items-center justify-center mb-4 group-hover:bg-blue-500/20 transition-colors">
                    <Laptop className="w-8 h-8 text-blue-400" />
                 </div>
                 <h3 className="text-white font-medium mb-1">Any Client</h3>
                 <p className="text-xs text-zinc-500">Web • Mobile • Console • CLI</p>
             </div>

             {/* Connection Line 1 */}
             <div className="relative flex-col lg:flex-row flex items-center justify-center -my-1 lg:my-0 lg:-mx-1 z-0">
                 {/* Desktop Horizontal Line */}
                 <div className="hidden lg:block w-24 h-[2px] bg-zinc-800 relative overflow-hidden">
                    <div className="absolute inset-0 bg-gradient-to-r from-transparent via-blue-500 to-transparent w-1/2 animate-[flow-h_2s_linear_infinite]" />
                 </div>
                 {/* Mobile Vertical Line */}
                 <div className="block lg:hidden h-16 w-[2px] bg-zinc-800 relative overflow-hidden">
                    <div className="absolute inset-0 bg-gradient-to-b from-transparent via-blue-500 to-transparent h-1/2 animate-[flow-v_2s_linear_infinite]" />
                 </div>
             </div>

             {/* 2. The Gateway: Rivet */}
             <div className="w-full max-w-[320px] p-8 rounded-2xl bg-zinc-900 border border-[#FF4500]/30 flex flex-col items-center text-center z-20 shadow-[0_0_50px_-10px_rgba(255,69,0,0.15)] relative">
                 <div className="absolute inset-0 bg-gradient-to-b from-[#FF4500]/5 to-transparent rounded-2xl pointer-events-none" />

                 <div className="relative w-20 h-20 rounded-full bg-[#FF4500]/10 border border-[#FF4500]/20 flex items-center justify-center mb-6">
                    <Zap className="w-10 h-10 text-[#FF4500] fill-[#FF4500]" />
                 </div>
                 <h3 className="relative text-2xl font-medium text-white mb-2">Rivet Cloud</h3>
                 <div className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-[#FF4500]/10 text-[#FF4500] border border-[#FF4500]/20">
                    Gateway & Orchestrator
                 </div>
             </div>

             {/* Connection Line 2 - The Branching System */}
             {/* This container manages the split lines */}
             <div className="relative flex-col lg:flex-row flex items-center justify-center z-0 w-full lg:w-auto">
                 
                 {/* Desktop: The Branch Connector */}
                 <div className="hidden lg:flex flex-col h-[300px] w-24 relative">
                    {/* Main Feed Line from Gateway */}
                    <div className="absolute left-[-2px] top-1/2 -translate-y-1/2 w-full h-[2px] bg-transparent">
                         <div className="absolute left-0 top-0 w-[2px] h-[2px] bg-[#FF4500]" /> 
                    </div>

                    {/* Top Branch - Added overflow-hidden */}
                    <div className="absolute top-[17%] left-0 w-full h-[2px] bg-zinc-800 overflow-hidden">
                        <div className="absolute inset-0 bg-gradient-to-r from-transparent via-[#FF4500] to-transparent w-1/2 animate-[flow-h_2s_linear_infinite_0.5s]" />
                    </div>

                    {/* Middle Branch - Added overflow-hidden */}
                    <div className="absolute top-1/2 -translate-y-1/2 left-0 w-full h-[2px] bg-zinc-800 overflow-hidden">
                        <div className="absolute inset-0 bg-gradient-to-r from-transparent via-[#FF4500] to-transparent w-1/2 animate-[flow-h_2s_linear_infinite_0.5s]" />
                    </div>

                    {/* Bottom Branch - Added overflow-hidden */}
                    <div className="absolute bottom-[17%] left-0 w-full h-[2px] bg-zinc-800 overflow-hidden">
                        <div className="absolute inset-0 bg-gradient-to-r from-transparent via-[#FF4500] to-transparent w-1/2 animate-[flow-h_2s_linear_infinite_0.5s]" />
                    </div>
                 </div>

                 {/* Mobile: Vertical Line */}
                 <div className="block lg:hidden h-16 w-[2px] bg-zinc-800 relative overflow-hidden">
                    <div className="absolute inset-0 bg-gradient-to-b from-transparent via-[#FF4500] to-transparent h-1/2 animate-[flow-v_2s_linear_infinite_0.5s]" />
                 </div>
             </div>

             {/* 3. Destination: User Backend */}
             <div className="w-full max-w-[280px] flex flex-col gap-6 relative z-20">
                {/* 1. Docker */}
                <div className="p-4 rounded-xl bg-zinc-900/50 border border-white/10 flex items-center gap-4 hover:border-white/20 transition-colors relative group h-[88px]">
                    <div className="p-2 rounded bg-white/5 text-zinc-400 group-hover:text-[#FF4500] group-hover:bg-[#FF4500]/10 transition-colors">
                        <Server className="w-5 h-5" />
                    </div>
                    <div>
                        <div className="text-sm font-medium text-white">Docker Containers</div>
                        <div className="text-[10px] text-zinc-500 font-mono">AWS ECS • Kubernetes</div>
                    </div>
                </div>

                {/* 2. Serverless */}
                <div className="p-4 rounded-xl bg-zinc-900/50 border border-white/10 flex items-center gap-4 hover:border-white/20 transition-colors relative group h-[88px]">
                    <div className="p-2 rounded bg-white/5 text-zinc-400 group-hover:text-[#FF4500] group-hover:bg-[#FF4500]/10 transition-colors">
                        <Cloud className="w-5 h-5" />
                    </div>
                    <div>
                        <div className="text-sm font-medium text-white">Serverless Functions</div>
                        <div className="text-[10px] text-zinc-500 font-mono">Vercel • Cloudflare</div>
                    </div>
                </div>

                {/* 3. Dedicated */}
                <div className="p-4 rounded-xl bg-zinc-900/50 border border-white/10 flex items-center gap-4 hover:border-white/20 transition-colors relative group h-[88px]">
                    <div className="p-2 rounded bg-white/5 text-zinc-400 group-hover:text-[#FF4500] group-hover:bg-[#FF4500]/10 transition-colors">
                        <Terminal className="w-5 h-5" />
                    </div>
                    <div>
                        <div className="text-sm font-medium text-white">Dedicated Servers</div>
                        <div className="text-[10px] text-zinc-500 font-mono">Bare Metal • EC2</div>
                    </div>
                </div>
             </div>

          </div>

          <style jsx>{`
            @keyframes flow-h {
                0% { transform: translateX(-100%); opacity: 0; }
                10% { opacity: 1; }
                90% { opacity: 1; }
                100% { transform: translateX(200%); opacity: 0; }
            }
            @keyframes flow-v {
                0% { transform: translateY(-100%); opacity: 0; }
                10% { opacity: 1; }
                90% { opacity: 1; }
                100% { transform: translateY(200%); opacity: 0; }
            }
          `}</style>
      </div>
    </div>
  </section>
);


const CloudFeatures = () => {
    const features = [
        { title: "Orchestration", desc: "The control plane handles actor placement, lifecycle management, and health checks across your cluster automatically.", icon: Command, color: "zinc" },
        { title: "Managed Persistence", desc: "State is persisted with strict serializability using FoundationDB, ensuring global consistency without the ops burden.", icon: HardDrive, color: "orange" },
        { title: "Multi-Region", desc: "Deploy actors across regions worldwide. Compute and state live together at the edge, delivering ultra-low latency responses.", icon: Globe, color: "blue" },
        { title: "Bring Your Own Compute", desc: "Run your business logic on AWS, Vercel, Railway, or bare metal. Rivet connects them all into a unified platform.", icon: Network, color: "zinc" },
        { title: "Serverless & Containers", desc: "Works with your serverless or container deployments, giving you the flexibility to choose the best runtime for your workload.", icon: Box, color: "orange" },
        { title: "Observability", desc: "Live state inspection, event monitoring, network inspector, and REPL for debugging and monitoring actors.", icon: Activity, color: "blue" }
    ];

    return (
        <section className="py-32 bg-zinc-950 border-t border-white/10">
             <div className="max-w-7xl mx-auto px-6">
                <div className="mb-20">
                    <h2 className="text-3xl font-medium text-white mb-6 tracking-tight">Platform Features</h2>
                    <p className="text-zinc-400 max-w-2xl text-lg leading-relaxed">Everything you need to run stateful workloads in production.</p>
                </div>
                <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
                    {features.map((f, i) => (
                        <FeatureCard key={i} title={f.title} description={f.desc} icon={f.icon} color={f.color} />
                    ))}
                </div>
             </div>
        </section>
    );
}

const SelfHostingComparison = () => {
  const burdens = [
    "Provisioning a Distributed Store",
    "Configuring Cross-Region VPC Peering",
    "Managing TLS Certificates & Rotation",
    "Scaling the Coordination Plane",
    "Handling Partition Tolerance",
    "Implementing Zero-Downtime Drains"
  ];

  return (
    <section className="py-24 bg-black border-t border-white/10">
      <div className="max-w-7xl mx-auto px-6">
        <div className="text-center mb-16">
            <h2 className="text-3xl font-medium text-white mb-4 tracking-tight">Compare Deployment Models</h2>
            <p className="text-zinc-400 max-w-2xl mx-auto text-lg leading-relaxed">
                Rivet is open source. Run it yourself for total control, or use Rivet Cloud for a hands-off experience.
            </p>
        </div>

        <div className="grid md:grid-cols-2 gap-8 max-w-5xl mx-auto">
            {/* Rivet Cloud Card */}
            <div className="p-8 rounded-2xl border border-[#FF4500]/30 bg-gradient-to-b from-[#FF4500]/10 to-transparent relative overflow-hidden flex flex-col backdrop-blur-sm">
                <div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-[#FF4500]/40 to-transparent" />
                <div className="mb-6 flex items-center gap-3">
                     <div className="p-3 bg-[#FF4500]/10 rounded-lg text-[#FF4500]">
                         <Cloud className="w-6 h-6" />
                     </div>
                     <h3 className="text-2xl font-medium text-white">Rivet Cloud</h3>
                </div>
                <p className="text-zinc-400 text-sm mb-8 h-12 leading-relaxed">
                    Managed cloud solution for personal projects to enterprise.
                </p>
                
                <div className="space-y-4 flex-grow">
                    <div className="flex items-center justify-between py-3 border-b border-[#FF4500]/10">
                        <span className="text-zinc-400 text-sm">Scaling</span>
                        <span className="text-white font-mono text-sm">Managed</span>
                    </div>
                    <div className="flex items-center justify-between py-3 border-b border-[#FF4500]/10">
                        <span className="text-zinc-400 text-sm">Database</span>
                        <span className="text-white font-mono text-sm">FoundationDB</span>
                    </div>
                    <div className="flex items-center justify-between py-3 border-b border-[#FF4500]/10">
                        <span className="text-zinc-400 text-sm">Networking</span>
                        <span className="text-white font-mono text-sm">Global Mesh</span>
                    </div>
                     <div className="flex items-center justify-between py-3 border-b border-[#FF4500]/10">
                        <span className="text-zinc-400 text-sm">Updates</span>
                        <span className="text-white font-mono text-sm">Automatic</span>
                    </div>
                </div>

                <a href="https://dashboard.rivet.dev"
                    className="mt-8 w-full py-3 bg-white text-black font-medium rounded-lg transition-colors hover:bg-zinc-200 text-center"
                >
                    Get Started
                </a>
            </div>

            {/* Open Source Card */}
            <div className="p-8 rounded-2xl border border-white/10 bg-white/[0.02] relative flex flex-col backdrop-blur-sm transition-all duration-300 hover:border-white/20 hover:bg-white/[0.05]">
                <div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent" />
                <div className="mb-6 flex items-center gap-3">
                     <div className="p-3 bg-white/5 rounded-lg text-zinc-400">
                         <Server className="w-6 h-6" />
                     </div>
                     <h3 className="text-2xl font-medium text-white">Open Source</h3>
                </div>
                <p className="text-zinc-400 text-sm mb-8 h-12 leading-relaxed">
                    Maximum control for air-gapped environments or specific compliance requirements.
                </p>
                
                <div className="space-y-4 flex-grow">
                    <div className="flex items-center justify-between py-3 border-b border-white/5">
                        <span className="text-zinc-500 text-sm">Scaling</span>
                        <span className="text-zinc-300 font-mono text-sm">You Manage</span>
                    </div>
                    <div className="flex items-center justify-between py-3 border-b border-white/5">
                        <span className="text-zinc-500 text-sm">Database</span>
                        <span className="text-zinc-300 font-mono text-sm">BYO (postgres or filesystem)</span>
                    </div>
                    <div className="flex items-center justify-between py-3 border-b border-white/5">
                        <span className="text-zinc-500 text-sm">Networking</span>
                        <span className="text-zinc-300 font-mono text-sm">Manual VPC</span>
                    </div>
                    <div className="flex items-center justify-between py-3 border-b border-white/5">
                        <span className="text-zinc-500 text-sm">Updates</span>
                        <span className="text-zinc-300 font-mono text-sm">Manual</span>
                    </div>
                </div>

                <a href="https://github.com/rivet-dev/rivet"
                    className="mt-8 w-full py-3 border border-white/10 bg-white/5 hover:bg-white/10 text-white font-medium rounded-lg transition-colors text-center"
                >
                    View on Github
                </a>
            </div>
        </div>
      </div>
    </section>
  )
}

const ComparisonTable = () => {
    const features = [
      { name: "Awake Actor Hours", free: "100,000 Cap", hobby: "400,000 Included", team: "400,000 Included", ent: "Custom" },
      { name: "Storage", free: "5GB", hobby: "5GB", team: "5GB", ent: "Custom" },
      { name: "Reads / mo", free: "200 Million", hobby: "25 Billion", team: "25 Billion", ent: "Custom" },
      { name: "Writes / mo", free: "5 Million", hobby: "50 Million", team: "50 Million", ent: "Custom" },
      { name: "Egress", free: "100GB", hobby: "1TB", team: "1TB", ent: "Custom" },
      { name: "Support", free: "Community", hobby: "Email", team: "Slack & Email", ent: "Slack & Email" },
      { name: "MFA", free: false, hobby: false, team: true, ent: true },
      { name: "Custom Regions", free: false, hobby: false, team: false, ent: true },
      { name: "SLA", free: false, hobby: false, team: false, ent: true },
      { name: "Audit Logs", free: false, hobby: false, team: false, ent: true },
      { name: "Custom Roles", free: false, hobby: false, team: false, ent: true },
      { name: "Device Tracking", free: false, hobby: false, team: false, ent: true },
      { name: "Volume Pricing", free: false, hobby: false, team: false, ent: true },
    ];
  
    const renderCell = (value) => {
      if (typeof value === 'boolean') {
        return value ? 
          <div className="flex justify-center"><Check className="w-5 h-5 text-[#FF4500]" /></div> : 
          <div className="flex justify-center"><div className="w-1.5 h-1.5 rounded-full bg-zinc-800" /></div>;
      }
      return <span className="text-sm text-zinc-300">{value}</span>;
    };
  
    return (
        <div className="mt-24">
            <h3 className="text-2xl font-medium text-white text-center mb-12 tracking-tight">Compare Plans</h3>
            <div className="overflow-x-auto">
                <table className="w-full min-w-[800px] border-collapse">
                    <thead>
                        <tr className="border-b border-white/10">
                            <th className="p-4 text-left text-sm font-medium text-zinc-500 w-1/4">Feature</th>
                            <th className="p-4 text-center text-sm font-medium text-white w-[18%]">Free</th>
                            <th className="p-4 text-center text-sm font-medium text-[#FF4500] w-[18%]">Hobby</th>
                            <th className="p-4 text-center text-sm font-medium text-white w-[18%]">Team</th>
                            <th className="p-4 text-center text-sm font-medium text-white w-[18%]">Enterprise</th>
                        </tr>
                    </thead>
                    <tbody>
                        {features.map((feature, i) => (
                            <tr key={i} className="border-b border-white/5 hover:bg-white/[0.02] transition-colors">
                                <td className="p-4 text-sm font-medium text-zinc-400">{feature.name}</td>
                                <td className="p-4 text-center">{renderCell(feature.free)}</td>
                                <td className="p-4 text-center bg-[#FF4500]/[0.02] border-x border-[#FF4500]/10">
                                    {renderCell(feature.hobby)}
                                </td>
                                <td className="p-4 text-center">{renderCell(feature.team)}</td>
                                <td className="p-4 text-center">{renderCell(feature.ent)}</td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            </div>
        </div>
    );
  };

const Pricing = () => {
    const [isCloud, setIsCloud] = useState(true);

    const cloudPlans = [
        {
            name: "Free",
            price: "$0",
            period: "/mo",
            desc: "For prototyping and small projects.",
            features: [
                "100,000 Awake Actor Hours Cap",
                "5GB Limit",
                "5 Million Writes /mo Limit",
                "200 Million Reads /mo Limit",
                "100GB Egress Limit",
                "Community Support"
            ],
            cta: "Get Started",
            highlight: false
        },
        {
            name: "Hobby",
            prefix: "From",
            price: "$20",
            period: "/mo + Usage",
            desc: "For scaling applications.",
            features: [
                "400,000 Awake Actor Hours Included",
                "25 Billion Reads /mo included",
                "50 Million Writes /mo included",
                "5GB Storage included",
                "1TB Egress included",
                "Email Support"
            ],
            cta: "Get Started",
            highlight: true
        },
        {
            name: "Team",
            prefix: "From",
            price: "$200",
            period: "/mo + Usage",
            desc: "For growing teams and businesses.",
            features: [
                "400,000 Awake Actor Hours Included",
                "25 Billion Reads /mo included",
                "50 Million Writes /mo included",
                "5GB Storage included",
                "1TB Egress included",
                "MFA",
                "Slack Support"
            ],
            cta: "Get Started",
            highlight: false
        },
        {
            name: "Enterprise",
            price: "Custom",
            period: "",
            desc: "For high-volume, mission-critical workloads.",
            features: [
                "Everything in Team",
                "Priority Support",
                "SLA",
                "OIDC SSO provider",
                "Audit Logs",
                "Custom Roles",
                "Device Tracking",
                "Volume Pricing"
            ],
            cta: "Contact",
            highlight: false
        }
    ];

    const selfHostedPlans = [
        {
            name: "Open Source",
            price: "Free",
            period: "Forever",
            desc: "Rivet is open source and free to use on your own infrastructure.",
            features: [
                "No usage limits",
                "Full source code access",
                "Community support"
            ],
            cta: "Get Started",
            highlight: false
        },
        {
            name: "Enterprise Support",
            price: "Custom",
            period: "",
            desc: "Get professional support and additional features for your self-hosted deployment.",
            features: [
                "Priority support",
                "SLA guarantees",
                "Custom integrations"
            ],
            cta: "Contact Sales",
            highlight: false
        }
    ];

    const plans = isCloud ? cloudPlans : selfHostedPlans;

    const usagePricing = [
        { resource: "Awake Actors", price: "$0.05", unit: "per 1k awake actor-hours" },
        { resource: "State Storage", price: "$0.40", unit: "per GB-month" },
        { resource: "Reads*", price: "$0.20", unit: "per million reads" },
        { resource: "Writes*", price: "$1", unit: "per million writes" },
        { resource: "Egress", price: "$0.15", unit: "per GB" },
        { resource: "Compute", price: "BYO", unit: "Paid to your provider" },
    ];

    return (
        <section id="pricing" className="py-32 bg-zinc-950 relative border-t border-white/10">
            <div className="max-w-7xl mx-auto px-6">
                <div className="text-center mb-16">
                    <h2 className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight">
                        {isCloud ? "Simple, Predictable Pricing" : "Rivet Self-Host"}
                    </h2>
                    <p className="text-zinc-400 mb-4 text-lg leading-relaxed">
                        {isCloud 
                            ? "Pay for coordination and state. Compute costs are billed directly by your chosen cloud provider."
                            : "Deploy Rivet on your own infrastructure."
                        }
                    </p>
                    
                    {/* Toggle */}
                    <div className="inline-flex bg-white/5 border border-white/10 p-1 rounded-lg">
                        <button 
                            onClick={() => setIsCloud(true)}
                            className={`px-6 py-2 rounded-md text-sm font-medium transition-all flex items-center gap-2 ${isCloud ? 'bg-white/10 text-white border border-white/20' : 'text-zinc-500 hover:text-zinc-300'}`}
                        >
                            <Cloud className="w-4 h-4" /> Cloud
                        </button>
                        <button 
                            onClick={() => setIsCloud(false)}
                            className={`px-6 py-2 rounded-md text-sm font-medium transition-all flex items-center gap-2 ${!isCloud ? 'bg-white/10 text-white border border-white/20' : 'text-zinc-500 hover:text-zinc-300'}`}
                        >
                            <Server className="w-4 h-4" /> Self-Hosted
                        </button>
                    </div>
                </div>

                <div className={`grid gap-4 items-start mb-24 transition-all duration-300 ${isCloud ? 'md:grid-cols-2 lg:grid-cols-4' : 'md:grid-cols-2 max-w-4xl mx-auto'}`}>
                    {plans.map((plan, idx) => (
                        <div 
                           key={idx} 
                           className={`group relative p-6 rounded-2xl border flex flex-col h-full transition-all duration-300 ${
                               plan.highlight 
                               ? 'border-[#FF4500]/50 bg-gradient-to-b from-[#FF4500]/10 to-transparent shadow-[0_0_40px_-10px_rgba(255,69,0,0.15)]' 
                               : 'border-white/10 bg-white/[0.02] hover:border-white/20 hover:bg-white/[0.05] backdrop-blur-sm'
                           }`}
                        >
                            {/* Highlight Effects for Pro Card */}
                            {plan.highlight && (
                                <>
                                    {/* Top Shine */}
                                    <div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-[#FF4500]/50 to-transparent" />
                                </>
                            )}
                            
                            {!plan.highlight && (
                                <div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent opacity-0 group-hover:opacity-100 transition-opacity" />
                            )}

                            <div className="flex-grow flex flex-col">
                                <h3 className="text-xl font-medium text-white mb-2">{plan.name}</h3>
                                
                                <div className="mb-6">
                                    {plan.prefix && <span className="text-zinc-500 text-xs font-medium uppercase tracking-wider block mb-1">{plan.prefix}</span>}
                                    <div className="flex items-baseline gap-1">
                                        <span className="text-3xl font-medium text-white tracking-tight">{plan.price}</span>
                                        {plan.period && <span className="text-zinc-500 text-xs font-mono ml-1">{plan.period}</span>}
                                    </div>
                                </div>
                                
                                {plan.desc && <p className="text-sm text-zinc-400 mb-6 min-h-[2.5rem] leading-relaxed">{plan.desc}</p>}

                                <div className="space-y-3 mb-8">
                                    {plan.features.map((feat, i) => (
                                        <div key={i} className="flex items-start gap-3 text-xs text-zinc-300">
                                            <Check className={`w-3 h-3 mt-0.5 ${plan.highlight ? 'text-[#FF4500]' : 'text-zinc-500'}`} />
                                            <span>{feat}</span>
                                        </div>
                                    ))}
                                </div>
                            </div>

                            <a href={plan.name === "Enterprise" || plan.name === "Enterprise Support" ? "/sales" : "https://dashboard.rivet.dev"}
                                className={`w-full py-3 rounded-lg text-sm font-medium transition-all text-center ${
                                    plan.highlight 
                                    ? 'bg-white text-black hover:bg-zinc-200' 
                                    : 'bg-white/5 text-white hover:bg-white/10 border border-white/10'
                                }`}
                            >
                                {plan.cta}
                            </a>
                        </div>
                    ))}
                </div>

                {/* Only show usage and comparison for Cloud */}
                {isCloud && (
                    <>
                        {/* Usage Pricing Section */}
                        <div className="border-t border-white/10 pt-16 mt-24">
                            <div className="text-center mb-12">
                                <h3 className="text-2xl font-medium text-white mb-2 tracking-tight">Usage Pricing</h3>
                                <p className="text-zinc-400 text-sm">Metered costs for scaling beyond plan limits.</p>
                            </div>
                            
                            <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
                                {usagePricing.map((item, i) => (
                                    <div key={i} className="group p-6 rounded-xl border border-white/10 bg-white/[0.02] flex flex-col items-center text-center hover:border-white/20 transition-colors backdrop-blur-sm">
                                        <div className="text-zinc-500 text-[10px] uppercase tracking-widest font-medium mb-3">{item.resource}</div>
                                        <div className={`text-2xl font-medium mb-1 ${item.price === "BYO" ? "text-zinc-500" : "text-white"}`}>{item.price}</div>
                                        <div className="text-zinc-500 text-xs">{item.unit}</div>
                                    </div>
                                ))}
                            </div>
                            <p className="text-zinc-500 text-xs mt-4 text-center">* Reads and writes to persisted actor state, not in-memory operations within an actor</p>
                        </div>

                        <ComparisonTable />
                    </>
                )}
            </div>
        </section>
    );
};


export default function PricingPageClient() {
  return (
    <div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-[#FF4500]/30 selection:text-orange-200">
      <main>
        <Hero />
        <CloudFeatures />
        <SelfHostingComparison />
        <Pricing />
      </main>
    </div>
  );
}
