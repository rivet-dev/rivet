"use client";

import React, { useState } from 'react';
import {
  Terminal,
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
import rivetLogoWhite from '@/images/rivet-logos/icon-white.svg';
import imgYC from '@/images/logos/yc.svg';
import imgA16z from '@/images/logos/a16z.svg';

// --- Page Sections ---

const Hero = () => (
  <section className="relative overflow-hidden pb-20 pt-32 md:pb-32 md:pt-48">
    <div className="mx-auto max-w-7xl px-6">
      <div className="flex flex-col items-center text-center">
        <div className="max-w-3xl mb-16">
          <h1 className="mb-6 text-4xl font-normal leading-[1.1] tracking-tight text-white md:text-6xl">
            Rivet Cloud
          </h1>

          <p className="mx-auto mb-8 max-w-2xl text-base leading-relaxed text-zinc-500">
             Rivet sits between your clients and your infrastructure. We manage the persistent connections, global routing, and actor state orchestrating your logic wherever it runs.
          </p>

          <div className="flex flex-col sm:flex-row items-center justify-center gap-3">
            <a href="https://dashboard.rivet.dev"
              className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200 w-full sm:w-auto"
            >
              Get Started for Free
              <ArrowRight className="h-4 w-4" />
            </a>
            <button
              onClick={() => document.getElementById('pricing')?.scrollIntoView({ behavior: 'smooth' })}
              className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white w-full sm:w-auto"
            >
              <CreditCard className="h-4 w-4" />
              See Pricing
            </button>
          </div>
        </div>

        {/* Main Diagram Container */}
        <div className="flex flex-col lg:flex-row items-center justify-center w-full">
             
             {/* 1. Source: Clients */}
             <div className="w-full max-w-[280px] p-6 rounded-2xl bg-zinc-900/50 border border-white/10 flex flex-col items-center text-center z-20 relative group hover:border-white/20 transition-colors">
                 <Laptop className="w-8 h-8 text-zinc-400 mb-4" />
                 <h3 className="text-white font-medium mb-1">Any Client</h3>
                 <p className="text-xs text-zinc-500">Web • Mobile • Console • CLI</p>
             </div>

             {/* Connection Line 1 - SVG style matching connection 2 */}
             <div className="relative flex-col lg:flex-row flex items-center justify-center -my-1 lg:my-0 z-0">
                 {/* Desktop Horizontal Line */}
                 <svg className="hidden lg:block w-20 h-[2px]" viewBox="0 0 80 2" fill="none" style={{ overflow: 'visible' }}>
                    <defs>
                      <linearGradient id="glowGradH1" gradientUnits="userSpaceOnUse" x1="0" y1="1" x2="80" y2="1">
                        <stop offset="0%" stopColor="#71717a" stopOpacity="0" />
                        <stop offset="50%" stopColor="#71717a" stopOpacity="1" />
                        <stop offset="100%" stopColor="#71717a" stopOpacity="0" />
                      </linearGradient>
                      <filter id="glowH1" x="-50%" y="-500%" width="200%" height="1100%">
                        <feGaussianBlur stdDeviation="3" result="blur" />
                        <feMerge>
                          <feMergeNode in="blur" />
                          <feMergeNode in="SourceGraphic" />
                        </feMerge>
                      </filter>
                    </defs>
                    {/* Base line */}
                    <line x1="0" y1="1" x2="80" y2="1" stroke="#27272a" strokeWidth="2" />
                    {/* Animated glow segment */}
                    <line x1="0" y1="1" x2="80" y2="1" stroke="url(#glowGradH1)" strokeWidth="2" filter="url(#glowH1)"
                      strokeDasharray="30 80" strokeDashoffset="30">
                      <animate attributeName="stroke-dashoffset" values="30;-80" dur="2s" repeatCount="indefinite" calcMode="linear" />
                    </line>
                 </svg>
                 {/* Mobile Vertical Line */}
                 <svg className="block lg:hidden w-[20px] h-16" viewBox="0 0 20 64" fill="none" style={{ overflow: 'visible' }}>
                    <defs>
                      <linearGradient id="glowGradV1" gradientUnits="userSpaceOnUse" x1="10" y1="0" x2="10" y2="64">
                        <stop offset="0%" stopColor="#71717a" stopOpacity="0" />
                        <stop offset="50%" stopColor="#71717a" stopOpacity="1" />
                        <stop offset="100%" stopColor="#71717a" stopOpacity="0" />
                      </linearGradient>
                      <filter id="glowV1" x="-50%" y="-50%" width="200%" height="200%">
                        <feGaussianBlur stdDeviation="3" result="blur" />
                        <feMerge>
                          <feMergeNode in="blur" />
                          <feMergeNode in="SourceGraphic" />
                        </feMerge>
                      </filter>
                    </defs>
                    {/* Base line */}
                    <line x1="10" y1="0" x2="10" y2="64" stroke="#27272a" strokeWidth="2" />
                    {/* Animated glow segment */}
                    <line x1="10" y1="0" x2="10" y2="64" stroke="url(#glowGradV1)" strokeWidth="2" filter="url(#glowV1)"
                      strokeDasharray="20 64" strokeDashoffset="20">
                      <animate attributeName="stroke-dashoffset" values="20;-64" dur="2s" repeatCount="indefinite" calcMode="linear" />
                    </line>
                 </svg>
             </div>

             {/* 2. The Gateway: Rivet */}
             <div className="w-full max-w-[320px] p-6 rounded-2xl bg-zinc-900/50 border border-white/10 flex flex-col items-center text-center z-20 relative">
                 <img src={rivetLogoWhite.src} alt="Rivet" className="w-8 h-8 mb-4" />
                 <h3 className="text-xl font-medium text-white mb-2">Rivet Cloud</h3>
                 <p className="text-xs text-zinc-500">Gateway & Orchestrator</p>
             </div>

             {/* Connection Line 2 - The Branching System with curved paths */}
             <div className="relative flex-col lg:flex-row flex items-center justify-center z-0 w-full lg:w-auto">

                 {/* Desktop: SVG curved connector with glowing lines */}
                 <svg className="hidden lg:block w-24 h-[260px]" viewBox="0 0 96 260" fill="none" style={{ overflow: 'visible' }}>
                    <defs>
                      <linearGradient id="glowGradH2" gradientUnits="userSpaceOnUse" x1="0" y1="130" x2="96" y2="130">
                        <stop offset="0%" stopColor="#71717a" stopOpacity="0" />
                        <stop offset="50%" stopColor="#71717a" stopOpacity="1" />
                        <stop offset="100%" stopColor="#71717a" stopOpacity="0" />
                      </linearGradient>
                      <filter id="glow2" x="-100%" y="-100%" width="300%" height="300%">
                        <feGaussianBlur stdDeviation="3" result="blur" />
                        <feMerge>
                          <feMergeNode in="blur" />
                          <feMergeNode in="SourceGraphic" />
                        </feMerge>
                      </filter>
                    </defs>

                    {/* Top curved path - base */}
                    <path d="M 0 128 Q 48 128 96 44" stroke="#27272a" strokeWidth="2" fill="none" />
                    {/* Top curved path - animated glow segment */}
                    <path d="M 0 128 Q 48 128 96 44" stroke="url(#glowGradH2)" strokeWidth="2" fill="none" filter="url(#glow2)"
                      strokeDasharray="30 130" strokeDashoffset="30">
                      <animate attributeName="stroke-dashoffset" values="30;-130" dur="2s" repeatCount="indefinite" calcMode="linear" />
                    </path>

                    {/* Middle straight path - base */}
                    <path d="M 0 130 L 96 130" stroke="#27272a" strokeWidth="2" fill="none" />
                    {/* Middle straight path - animated glow segment */}
                    <path d="M 0 130 L 96 130" stroke="url(#glowGradH2)" strokeWidth="2" fill="none" filter="url(#glow2)"
                      strokeDasharray="30 96" strokeDashoffset="30">
                      <animate attributeName="stroke-dashoffset" values="30;-96" dur="2s" repeatCount="indefinite" calcMode="linear" begin="0.3s" />
                    </path>

                    {/* Bottom curved path - base */}
                    <path d="M 0 132 Q 48 132 96 216" stroke="#27272a" strokeWidth="2" fill="none" />
                    {/* Bottom curved path - animated glow segment */}
                    <path d="M 0 132 Q 48 132 96 216" stroke="url(#glowGradH2)" strokeWidth="2" fill="none" filter="url(#glow2)"
                      strokeDasharray="30 130" strokeDashoffset="30">
                      <animate attributeName="stroke-dashoffset" values="30;-130" dur="2s" repeatCount="indefinite" calcMode="linear" begin="0.6s" />
                    </path>
                 </svg>

                 {/* Mobile: Vertical Line */}
                 <svg className="block lg:hidden w-[20px] h-16" viewBox="0 0 20 64" fill="none" style={{ overflow: 'visible' }}>
                    <defs>
                      <linearGradient id="glowGradV2" gradientUnits="userSpaceOnUse" x1="10" y1="0" x2="10" y2="64">
                        <stop offset="0%" stopColor="#71717a" stopOpacity="0" />
                        <stop offset="50%" stopColor="#71717a" stopOpacity="1" />
                        <stop offset="100%" stopColor="#71717a" stopOpacity="0" />
                      </linearGradient>
                      <filter id="glowV2" x="-50%" y="-50%" width="200%" height="200%">
                        <feGaussianBlur stdDeviation="3" result="blur" />
                        <feMerge>
                          <feMergeNode in="blur" />
                          <feMergeNode in="SourceGraphic" />
                        </feMerge>
                      </filter>
                    </defs>
                    {/* Base line */}
                    <line x1="10" y1="0" x2="10" y2="64" stroke="#27272a" strokeWidth="2" />
                    {/* Animated glow segment */}
                    <line x1="10" y1="0" x2="10" y2="64" stroke="url(#glowGradV2)" strokeWidth="2" filter="url(#glowV2)"
                      strokeDasharray="20 64" strokeDashoffset="20">
                      <animate attributeName="stroke-dashoffset" values="20;-64" dur="2s" repeatCount="indefinite" calcMode="linear" begin="0.5s" />
                    </line>
                 </svg>
             </div>

             {/* 3. Destination: User Backend */}
             <div className="w-full max-w-[280px] flex flex-col gap-4 relative z-20">
                {/* 1. Docker */}
                <div className="p-4 rounded-xl bg-zinc-900/50 border border-white/10 flex items-center gap-4 hover:border-white/20 transition-colors">
                    <Server className="w-5 h-5 text-zinc-400" />
                    <div>
                        <div className="text-sm font-medium text-white">Docker Containers</div>
                        <div className="text-[10px] text-zinc-500 font-mono">AWS ECS • Kubernetes</div>
                    </div>
                </div>

                {/* 2. Serverless */}
                <div className="p-4 rounded-xl bg-zinc-900/50 border border-white/10 flex items-center gap-4 hover:border-white/20 transition-colors">
                    <Cloud className="w-5 h-5 text-zinc-400" />
                    <div>
                        <div className="text-sm font-medium text-white">Serverless Functions</div>
                        <div className="text-[10px] text-zinc-500 font-mono">Vercel • Cloudflare</div>
                    </div>
                </div>

                {/* 3. Dedicated */}
                <div className="p-4 rounded-xl bg-zinc-900/50 border border-white/10 flex items-center gap-4 hover:border-white/20 transition-colors">
                    <Terminal className="w-5 h-5 text-zinc-400" />
                    <div>
                        <div className="text-sm font-medium text-white">Dedicated Servers</div>
                        <div className="text-[10px] text-zinc-500 font-mono">Bare Metal • EC2</div>
                    </div>
                </div>
             </div>

          </div>

      </div>
    </div>
  </section>
);


const CloudFeatures = () => {
    const features = [
        { title: "Orchestration", desc: "The control plane handles actor placement, lifecycle management, and health checks across your cluster automatically.", icon: Command },
        { title: "Managed Persistence", desc: "State is persisted with strict serializability using FoundationDB, ensuring global consistency without the ops burden.", icon: HardDrive },
        { title: "Multi-Region", desc: "Deploy actors across regions worldwide. Compute and state live together at the edge, delivering ultra-low latency responses.", icon: Globe },
        { title: "Bring Your Own Compute", desc: "Run your business logic on AWS, Vercel, Railway, or bare metal. Rivet connects them all into a unified platform.", icon: Network },
        { title: "Serverless & Containers", desc: "Works with your serverless or container deployments, giving you the flexibility to choose the best runtime for your workload.", icon: Box },
        { title: "Observability", desc: "Live state inspection, event monitoring, network inspector, and REPL for debugging and monitoring actors.", icon: Activity }
    ];

    return (
        <section className="border-t border-white/10 py-48">
            <div className="mx-auto max-w-7xl px-6">
                <div className="flex flex-col gap-12">
                    <div className="max-w-xl">
                        <h2 className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl">Platform Features</h2>
                        <p className="text-base leading-relaxed text-zinc-500">Everything you need to run stateful workloads in production.</p>
                    </div>
                    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
                        {features.map((f, i) => (
                            <div key={i} className="border-t border-white/10 pt-6">
                                <div className="mb-3 text-zinc-500">
                                    <f.icon className="h-4 w-4" />
                                </div>
                                <h3 className="mb-1 text-sm font-normal text-white">{f.title}</h3>
                                <p className="text-sm leading-relaxed text-zinc-500">{f.desc}</p>
                            </div>
                        ))}
                    </div>
                </div>
            </div>
        </section>
    );
}

const SelfHostingComparison = () => {
  return (
    <section className="border-t border-white/10 py-48">
      <div className="mx-auto max-w-7xl px-6">
        <div className="flex flex-col gap-12">
          <div className="max-w-xl">
            <h2 className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl">Compare Deployment Models</h2>
            <p className="text-base leading-relaxed text-zinc-500">
                Rivet is open source. Run it yourself for total control, or use Rivet Cloud for a hands-off experience.
            </p>
          </div>

          <div className="grid md:grid-cols-2 gap-8">
              {/* Rivet Cloud Card */}
              <div className="rounded-lg border border-[#FF4500]/20 bg-black p-8 flex flex-col">
                  <div className="mb-6 flex items-center gap-3">
                       <img src={rivetLogoWhite.src} alt="Rivet" className="h-10 w-10" />
                       <h3 className="text-lg font-normal text-white">Rivet Cloud</h3>
                  </div>
                  <p className="text-sm leading-relaxed text-zinc-400 mb-8">
                      Managed cloud solution for personal projects to enterprise.
                  </p>

                  <div className="space-y-4 flex-grow font-mono text-sm">
                      <div className="flex items-center justify-between py-3 border-b border-white/5">
                          <span className="text-zinc-500">Scaling</span>
                          <span className="text-white">Managed</span>
                      </div>
                      <div className="flex items-center justify-between py-3 border-b border-white/5">
                          <span className="text-zinc-500">Database</span>
                          <span className="text-white">FoundationDB</span>
                      </div>
                      <div className="flex items-center justify-between py-3 border-b border-white/5">
                          <span className="text-zinc-500">Networking</span>
                          <span className="text-white">Global Mesh</span>
                      </div>
                       <div className="flex items-center justify-between py-3 border-b border-white/5">
                          <span className="text-zinc-500">Updates</span>
                          <span className="text-white">Automatic</span>
                      </div>
                  </div>

                  <a href="https://dashboard.rivet.dev"
                      className="mt-8 w-full rounded-md bg-white py-3 text-center text-sm font-medium text-black transition-colors hover:bg-zinc-200"
                  >
                      Get Started
                  </a>
              </div>

              {/* Open Source Card */}
              <div className="rounded-lg border border-white/10 bg-black p-8 flex flex-col">
                  <div className="mb-6 flex items-center gap-3">
                       <Server className="h-6 w-6 text-zinc-500" />
                       <h3 className="text-lg font-normal text-white">Open Source</h3>
                  </div>
                  <p className="text-sm leading-relaxed text-zinc-500 mb-8">
                      Maximum control for air-gapped environments or specific compliance requirements.
                  </p>

                  <div className="space-y-4 flex-grow font-mono text-sm">
                      <div className="flex items-center justify-between py-3 border-b border-white/5">
                          <span className="text-zinc-500">Scaling</span>
                          <span className="text-zinc-300">You Manage</span>
                      </div>
                      <div className="flex items-center justify-between py-3 border-b border-white/5">
                          <span className="text-zinc-500">Database</span>
                          <span className="text-zinc-300">BYO (postgres or filesystem)</span>
                      </div>
                      <div className="flex items-center justify-between py-3 border-b border-white/5">
                          <span className="text-zinc-500">Networking</span>
                          <span className="text-zinc-300">Manual VPC</span>
                      </div>
                      <div className="flex items-center justify-between py-3 border-b border-white/5">
                          <span className="text-zinc-500">Updates</span>
                          <span className="text-zinc-300">Manual</span>
                      </div>
                  </div>

                  <a href="https://github.com/rivet-dev/rivet"
                      className="mt-8 w-full rounded-md border border-white/10 py-3 text-center text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white"
                  >
                      View on Github
                  </a>
              </div>
          </div>
        </div>
      </div>
    </section>
  )
}

const ComparisonTable = () => {
    const features = [
      { name: "Awake Actor Hours", free: "100,000", hobby: "400,000", team: "400,000", ent: "Custom" },
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
          <div className="flex justify-center"><Check className="h-4 w-4 text-[#FF4500]" /></div> :
          <div className="flex justify-center"><div className="h-1.5 w-1.5 rounded-full bg-zinc-700" /></div>;
      }
      return <span className="text-sm text-zinc-300">{value}</span>;
    };

    return (
        <div className="mt-24 border-t border-white/10 pt-16">
            <h3 className="mb-12 text-2xl font-normal tracking-tight text-white">Compare Plans</h3>
            <div className="overflow-x-auto">
                <table className="w-full min-w-[800px] border-collapse">
                    <thead>
                        <tr className="border-b border-white/10">
                            <th className="p-4 text-left text-sm font-medium uppercase tracking-wider text-zinc-500 w-1/4">Feature</th>
                            <th className="p-4 text-center text-sm font-medium text-white w-[18%]">Free</th>
                            <th className="p-4 text-center text-sm font-medium text-[#FF4500] w-[18%]">Hobby</th>
                            <th className="p-4 text-center text-sm font-medium text-white w-[18%]">Team</th>
                            <th className="p-4 text-center text-sm font-medium text-white w-[18%]">Enterprise</th>
                        </tr>
                    </thead>
                    <tbody>
                        {features.map((feature, i) => (
                            <tr key={i} className="border-b border-white/5">
                                <td className="p-4 text-sm text-zinc-400">{feature.name}</td>
                                <td className="p-4 text-center">{renderCell(feature.free)}</td>
                                <td className="p-4 text-center">{renderCell(feature.hobby)}</td>
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
                "100,000 Awake Actor Hours /mo limit",
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
        <section id="pricing" className="border-t border-white/10 py-48">
            <div className="mx-auto max-w-7xl px-6">
                <div className="flex flex-col gap-12">
                    <div className="flex flex-col items-center text-center">
                        <h2 className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl">
                            {isCloud ? "Simple, Predictable Pricing" : "Rivet Self-Host"}
                        </h2>
                        <p className="mb-6 max-w-lg text-base leading-relaxed text-zinc-500">
                            {isCloud
                                ? "Pay for coordination and state. Compute costs are billed directly by your chosen cloud provider."
                                : "Deploy Rivet on your own infrastructure."
                            }
                        </p>

                        {/* Toggle */}
                        <div className="inline-flex rounded-md border border-white/10 p-1">
                            <button
                                onClick={() => setIsCloud(true)}
                                className={`flex items-center gap-2 rounded px-4 py-2 text-sm transition-all ${isCloud ? 'bg-white/10 text-white' : 'text-zinc-500 hover:text-zinc-300'}`}
                            >
                                <Cloud className="h-4 w-4" /> Cloud
                            </button>
                            <button
                                onClick={() => setIsCloud(false)}
                                className={`flex items-center gap-2 rounded px-4 py-2 text-sm transition-all ${!isCloud ? 'bg-white/10 text-white' : 'text-zinc-500 hover:text-zinc-300'}`}
                            >
                                <Server className="h-4 w-4" /> Self-Hosted
                            </button>
                        </div>
                    </div>

                    <div className={`grid gap-6 ${isCloud ? 'md:grid-cols-2 lg:grid-cols-4' : 'md:grid-cols-2 max-w-4xl mx-auto'}`}>
                        {plans.map((plan, idx) => (
                            <div
                               key={idx}
                               className={`flex flex-col rounded-lg border p-6 ${
                                   plan.highlight
                                   ? 'border-[#FF4500]/30 bg-black'
                                   : 'border-white/10 bg-black'
                               }`}
                            >
                                <div className="flex-grow flex flex-col">
                                    <h3 className="text-lg font-normal text-white mb-2">{plan.name}</h3>

                                    <div className="mb-6">
                                        {plan.prefix && <span className="text-zinc-500 text-xs font-medium uppercase tracking-wider block mb-1">{plan.prefix}</span>}
                                        <div className="flex items-baseline gap-1">
                                            <span className="text-3xl font-normal tracking-tight text-white">{plan.price}</span>
                                            {plan.period && <span className="ml-1 font-mono text-xs text-zinc-500">{plan.period}</span>}
                                        </div>
                                    </div>

                                    {plan.desc && <p className="mb-6 min-h-[2.5rem] text-sm leading-relaxed text-zinc-400">{plan.desc}</p>}

                                    <div className="mb-8 space-y-3">
                                        {plan.features.map((feat, i) => (
                                            <div key={i} className="flex items-start gap-3 text-xs text-zinc-300">
                                                <Check className={`mt-0.5 h-3 w-3 ${plan.highlight ? 'text-[#FF4500]' : 'text-zinc-500'}`} />
                                                <span>{feat}</span>
                                            </div>
                                        ))}
                                    </div>
                                </div>

                                <a href={plan.name === "Enterprise" || plan.name === "Enterprise Support" ? "/sales" : "https://dashboard.rivet.dev"}
                                    className={`w-full rounded-md py-3 text-center text-sm font-medium transition-colors ${
                                        plan.highlight
                                        ? 'bg-white text-black hover:bg-zinc-200'
                                        : 'border border-white/10 text-zinc-300 hover:border-white/20 hover:text-white'
                                    }`}
                                >
                                    {plan.cta}
                                </a>
                            </div>
                        ))}
                    </div>

                    {/* YC & a16z Speedrun Callout */}
                    {isCloud && (
                        <div className="mt-12 rounded-lg border border-white/10 p-6">
                            <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-6">
                                <div>
                                    <p className="text-base text-white mb-2">Startup Deal: 50% off for 12 months</p>
                                    <div className="flex flex-wrap items-center gap-2 text-sm text-zinc-500">
                                        <span>For</span>
                                        <div className="flex items-center gap-2 rounded-full border border-white/10 px-3 py-1.5 text-xs text-zinc-400">
                                            <img src={imgYC.src} alt="Y Combinator" className="h-4 w-auto" />
                                            <span>Y Combinator</span>
                                        </div>
                                        <span>and</span>
                                        <div className="flex items-center gap-2 rounded-full border border-white/10 px-3 py-1.5 text-xs text-zinc-400">
                                            <img src={imgA16z.src} alt="a16z" className="h-3 w-auto" />
                                            <span>a16z Speedrun</span>
                                        </div>
                                        <span>companies</span>
                                    </div>
                                </div>
                                <a
                                    href="/startups"
                                    className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white"
                                >
                                    Claim the deal
                                    <ArrowRight className="h-4 w-4" />
                                </a>
                            </div>
                        </div>
                    )}

                    {/* Only show usage and comparison for Cloud */}
                    {isCloud && (
                        <>
                            {/* Usage Pricing Section */}
                            <div className="border-t border-white/10 pt-16 mt-12">
                                <h3 className="mb-3 text-2xl font-normal tracking-tight text-white">Usage Pricing</h3>
                                <p className="mb-8 text-base leading-relaxed text-zinc-500">Metered costs for scaling beyond plan limits.</p>

                                <div className="grid grid-cols-2 md:grid-cols-3 gap-6">
                                    {usagePricing.map((item, i) => (
                                        <div key={i} className="border-t border-white/10 pt-6">
                                            <div className="mb-2 text-xs font-medium uppercase tracking-wider text-zinc-500">{item.resource}</div>
                                            <div className={`mb-1 text-2xl font-normal ${item.price === "BYO" ? "text-zinc-500" : "text-white"}`}>{item.price}</div>
                                            <div className="text-xs text-zinc-500">{item.unit}</div>
                                        </div>
                                    ))}
                                </div>
                                <p className="mt-6 text-xs text-zinc-500">* Reads and writes to persisted actor state, not in-memory operations within an actor</p>
                            </div>

                            <ComparisonTable />
                        </>
                    )}
                </div>
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
