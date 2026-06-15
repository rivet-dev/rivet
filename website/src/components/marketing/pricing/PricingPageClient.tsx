"use client";

import { useState } from 'react';
import {
  ArrowRight,
  Check,
  Server,
  ShieldCheck,
  Cpu,
  MemoryStick,
  Clock
} from 'lucide-react';
import rivetLogoWhite from '@/images/rivet-logos/icon-white.svg';
import imgYC from '@/images/logos/yc.svg';
import imgA16z from '@/images/logos/a16z.svg';
import { SECTION_H2_CLASS, SUBTITLE_CLASS } from '@/components/marketing/typography';
import { InkPanel } from '@/components/marketing/editorial/InkPanel';

// --- Page Sections ---

const SelfHostingComparison = () => {
  const cloudSpecs = [
    { label: 'Scaling', value: 'Managed' },
    { label: 'Database', value: 'FoundationDB' },
    { label: 'Networking', value: 'Global Mesh' },
    { label: 'Updates', value: 'Automatic' },
  ];

  const selfHostedSpecs = [
    { label: 'Scaling', value: 'You Manage' },
    { label: 'Database', value: 'BYO (postgres or filesystem)' },
    { label: 'Networking', value: 'Manual VPC' },
    { label: 'Updates', value: 'Manual' },
  ];

  const enterpriseSpecs = [
    { label: 'Scaling', value: 'You Manage' },
    { label: 'Database', value: 'FoundationDB' },
    { label: 'Networking', value: 'VPC & air-gapped' },
    { label: 'Updates', value: 'Guided' },
  ];

  return (
    <section className="border-t border-ink/10 py-16 md:py-32">
      <div className="mx-auto max-w-7xl px-6">
        <div className="flex flex-col gap-12">
          <div className="max-w-xl">
            <h2 className={`mb-2 ${SECTION_H2_CLASS}`}>Compare Deployment Models</h2>
            <p className={SUBTITLE_CLASS}>
                Rivet is open source. Use Rivet Cloud for managed infrastructure, or self-host in your VPC, your customers' environments, or air-gapped networks.
            </p>
          </div>

          <div className="grid gap-8 md:grid-cols-3">
              {/* Rivet Cloud Card */}
              <div className="flex flex-col border border-ink/10 bg-white/55 p-7">
                  <div className="mb-6 flex items-center gap-3">
                       <img src={rivetLogoWhite.src} alt="Rivet" className="h-9 w-9 invert" />
                       <h3 className="text-lg font-medium text-ink">Rivet Cloud</h3>
                  </div>
                  <p className="mb-8 text-sm leading-relaxed text-ink-soft">
                      Managed cloud solution for personal projects to large orgs.
                  </p>

                  <div className="flex-grow font-mono text-sm">
                      {cloudSpecs.map(({ label, value }) => (
                          <div key={label} className="flex items-center justify-between gap-4 border-b border-ink/10 py-3.5">
                              <span className="text-[11px] uppercase tracking-[0.16em] text-ink-faint">{label}</span>
                              <span className="text-right text-ink">{value}</span>
                          </div>
                      ))}
                  </div>

                  <a href="https://dashboard.rivet.dev"
                      className="mt-8 w-full rounded-md bg-ink py-3 text-center text-sm font-medium text-cream transition-colors hover:bg-ink/85"
                  >
                      Get Started
                  </a>
              </div>

              {/* Self-Hosted Card */}
              <InkPanel className="flex flex-col [&>div:first-child]:flex-grow">
                  <div className="flex h-full flex-col p-7">
                      <div className="mb-6 flex items-center gap-3">
                           <Server className="h-6 w-6 text-cream" />
                           <h3 className="text-lg font-medium text-cream">Self-Hosted</h3>
                      </div>
                      <p className="mb-8 text-sm leading-relaxed text-cream/60">
                          Run the open-source version of Rivet on your own infrastructure. Apache 2.0, no usage limits, full source access.
                      </p>

                      <div className="flex-grow font-mono text-sm">
                          {selfHostedSpecs.map(({ label, value }) => (
                              <div key={label} className="flex items-center justify-between gap-4 border-b border-cream/10 py-3.5">
                                  <span className="text-[11px] uppercase tracking-[0.16em] text-cream/50">{label}</span>
                                  <span className="text-right text-cream">{value}</span>
                              </div>
                          ))}
                      </div>

                      <a href="https://github.com/rivet-dev/rivet"
                          className="mt-8 w-full rounded-md border border-cream/20 py-3 text-center text-sm text-cream/85 transition-colors hover:border-cream/40 hover:text-cream"
                      >
                          View on GitHub
                      </a>
                      <p className="mt-4 text-center text-xs text-cream/50">
                          High-touch deployment? <a href="/enterprise" className="text-cream/80 transition-colors hover:text-cream">Rivet for Enterprise</a>
                      </p>
                  </div>
              </InkPanel>

              {/* Enterprise Edition Card */}
              <InkPanel className="flex flex-col [&>div:first-child]:flex-grow">
                  <div className="flex h-full flex-col p-7">
                      <div className="mb-6 flex items-center gap-3">
                           <ShieldCheck className="h-6 w-6 text-cream" />
                           <h3 className="text-lg font-medium text-cream">Enterprise Edition</h3>
                      </div>
                      <p className="mb-8 text-sm leading-relaxed text-cream/60">
                          Production-grade closed-source features on top of open-source Rivet. Orchestration, multi-tenancy, and compliance to run at enterprise scale.
                      </p>

                      <div className="flex-grow font-mono text-sm">
                          {enterpriseSpecs.map(({ label, value }) => (
                              <div key={label} className="flex items-center justify-between gap-4 border-b border-cream/10 py-3.5">
                                  <span className="text-[11px] uppercase tracking-[0.16em] text-cream/50">{label}</span>
                                  <span className="text-right text-cream">{value}</span>
                              </div>
                          ))}
                      </div>

                      <a href="/sales"
                          className="mt-8 w-full rounded-md border border-cream/20 py-3 text-center text-sm text-cream/85 transition-colors hover:border-cream/40 hover:text-cream"
                      >
                          Contact Sales
                      </a>
                  </div>
              </InkPanel>
          </div>
        </div>
      </div>
    </section>
  )
}

const ComparisonTable = () => {
    const features = [
      { name: "Awake Actor Hours", free: "100,000 max", hobby: "400,000 included", team: "400,000 included" },
      { name: "Compute", free: "$5 max", hobby: "Usage-based", team: "Usage-based" },
      { name: "Max vCPU", free: "1", hobby: "8", team: "8" },
      { name: "Storage", free: "5GB max", hobby: "5GB included", team: "5GB included" },
      { name: "Reads / mo", free: "200 Million max", hobby: "25 Billion included", team: "25 Billion included" },
      { name: "Writes / mo", free: "5 Million max", hobby: "50 Million included", team: "50 Million included" },
      { name: "Egress", free: "100GB max", hobby: "1TB included", team: "1TB included" },
      { name: "Support", free: "Community", hobby: "Email", team: "Slack & Email" },
      { name: "MFA", free: false, hobby: false, team: true },
    ];

    const renderCell = (value) => {
      if (typeof value === 'boolean') {
        return value ?
          <div className="flex justify-center"><Check className="h-4 w-4 text-pine" /></div> :
          <div className="flex justify-center"><div className="h-1.5 w-1.5 rounded-full bg-ink/20" /></div>;
      }
      return <span className="text-sm text-ink-soft">{value}</span>;
    };

    return (
        <div className="mt-24 border-t border-ink/10 pt-16">
            <h3 className="mb-12 text-2xl font-medium tracking-[-0.015em] text-ink">Compare Cloud Plans</h3>
            <div className="overflow-x-auto">
                <table className="w-full min-w-[800px] border-collapse">
                    <thead>
                        <tr className="border-b border-ink/15">
                            <th className="w-1/4 p-4 text-left font-mono text-[11px] font-medium uppercase tracking-[0.16em] text-ink-faint">Feature</th>
                            <th className="w-[18%] p-4 text-center text-sm font-medium text-ink">Free</th>
                            <th className="w-[18%] p-4 text-center text-sm font-medium text-pine">Hobby</th>
                            <th className="w-[18%] p-4 text-center text-sm font-medium text-ink">Team</th>
                        </tr>
                    </thead>
                    <tbody>
                        {features.map((feature, i) => (
                            <tr key={i} className="border-b border-ink/10">
                                <td className="p-4 text-sm text-ink-soft">{feature.name}</td>
                                <td className="p-4 text-center">{renderCell(feature.free)}</td>
                                <td className="p-4 text-center">{renderCell(feature.hobby)}</td>
                                <td className="p-4 text-center">{renderCell(feature.team)}</td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            </div>
            <p className="mt-6 text-xs text-ink-faint">
                Free plan values are hard monthly limits. Hobby and Team include the listed amounts, then bill per usage; paid compute has no included amount and is billed per usage.
            </p>
        </div>
    );
  };

// Rivet Compute pricing. Cost is billed per active second based on each actor's
// configured CPU and memory:
//   cost = active_seconds × (vcpus × CPU_PER_VCPU_SECOND + memory_gib × MEMORY_PER_GIB_SECOND)
// One vCPU is half a physical core. The Free plan is limited to 1 vCPU; paid plans
// allow up to 8 vCPU.
const COMPUTE = {
    cpuPerVcpuSecond: 0.000033,
    memoryPerGibSecond: 0.0000029,
    maxVcpu: 8,
    freeMaxVcpu: 1,
};

// Valid compute shapes. vCPU is continuous from 0.08 to 1, then exactly 2, 4,
// or 8. Memory ranges from 128 MiB to 4096 MiB (4 GiB).
const VCPU_STEPS = [0.08, 0.25, 0.5, 1, 2, 4, 8];
const MEMORY_STEPS = [128, 256, 512, 1024, 2048, 4096]; // MiB

const usd = (n: number, decimals = 2) =>
    `$${n.toLocaleString("en-US", { minimumFractionDigits: decimals, maximumFractionDigits: decimals })}`;

const ComputeCalculator = () => {
    const [vcpuIdx, setVcpuIdx] = useState(3); // 1 vCPU
    const [memIdx, setMemIdx] = useState(2); // 512 MiB
    const [hours, setHours] = useState(100);

    const vcpus = VCPU_STEPS[vcpuIdx];
    const memoryMib = MEMORY_STEPS[memIdx];
    const cpuPerSec = vcpus * COMPUTE.cpuPerVcpuSecond;
    const memPerSec = (memoryMib / 1024) * COMPUTE.memoryPerGibSecond;
    const perSecond = cpuPerSec + memPerSec;
    const monthly = perSecond * hours * 3600;

    const memLabel = memoryMib >= 1024 ? `${memoryMib / 1024} GiB` : `${memoryMib} MiB`;

    return (
        <div className="border-t border-ink/10 pt-16">
            <h3 className="mb-3 text-2xl font-medium tracking-[-0.015em] text-ink">Estimate your compute</h3>
            <p className="mb-8 max-w-2xl text-base leading-relaxed text-ink-soft">
                Run your actors and applications on Rivet Compute and pay only for the seconds they are active.
                Costs scale with the CPU and memory you configure.
            </p>

            <div className="grid gap-6 lg:grid-cols-2">
                {/* Controls sit on porcelain: this is interactive input UI. */}
                <div className="space-y-8 border border-ink/10 bg-white/55 p-8">
                    {/* vCPU */}
                    <div>
                        <div className="mb-3 flex items-center justify-between">
                            <span className="flex items-center gap-2 text-sm text-ink-soft">
                                <Cpu className="h-4 w-4 text-ink-faint" /> vCPU
                            </span>
                            <span className="font-mono text-sm text-ink">{vcpus}</span>
                        </div>
                        <input
                            type="range"
                            min={0}
                            max={VCPU_STEPS.length - 1}
                            step={1}
                            value={vcpuIdx}
                            onChange={(e) => setVcpuIdx(Number(e.target.value))}
                            className="w-full accent-pine"
                        />
                        <p className="mt-2 text-xs text-ink-faint">
                            1 vCPU = half a physical core. 0.08–1 vCPU, or exactly 2, 4, or 8.
                        </p>
                    </div>

                    {/* Memory */}
                    <div>
                        <div className="mb-3 flex items-center justify-between">
                            <span className="flex items-center gap-2 text-sm text-ink-soft">
                                <MemoryStick className="h-4 w-4 text-ink-faint" /> Memory
                            </span>
                            <span className="font-mono text-sm text-ink">{memLabel}</span>
                        </div>
                        <input
                            type="range"
                            min={0}
                            max={MEMORY_STEPS.length - 1}
                            step={1}
                            value={memIdx}
                            onChange={(e) => setMemIdx(Number(e.target.value))}
                            className="w-full accent-pine"
                        />
                        <p className="mt-2 text-xs text-ink-faint">
                            128 MiB to 4 GiB.
                        </p>
                    </div>

                    {/* Active time */}
                    <div>
                        <div className="mb-3 flex items-center justify-between">
                            <span className="flex items-center gap-2 text-sm text-ink-soft">
                                <Clock className="h-4 w-4 text-ink-faint" /> Active hours / month
                            </span>
                            <span className="font-mono text-sm text-ink">{hours >= 730 ? "Always on" : `${hours} h`}</span>
                        </div>
                        <input
                            type="range"
                            min={1}
                            max={730}
                            step={1}
                            value={hours}
                            onChange={(e) => setHours(Number(e.target.value))}
                            className="w-full accent-pine"
                        />
                        <p className="mt-2 text-xs text-ink-faint">
                            Sleeping actors are not billed for compute.
                        </p>
                    </div>
                </div>

                {/* Result is the data moment: render it on the ink plate so the
                    estimate reads as output, mirroring the Self-Hosted panel. */}
                <InkPanel className="flex flex-col p-8">
                    <span className="font-mono text-[11px] font-medium uppercase tracking-[0.16em] text-sage">Estimated compute</span>
                    <div className="mt-2 flex items-baseline gap-2">
                        <span className="text-4xl font-medium tracking-[-0.015em] text-cream">{usd(monthly)}</span>
                        <span className="font-mono text-xs text-cream/50">/mo</span>
                    </div>

                    <div className="mt-8 space-y-3 font-mono text-sm">
                        <div className="flex items-center justify-between border-b border-cream/10 py-2">
                            <span className="text-cream/50">CPU ({vcpus} vCPU)</span>
                            <span className="text-cream/85">{usd(cpuPerSec * hours * 3600)}</span>
                        </div>
                        <div className="flex items-center justify-between border-b border-cream/10 py-2">
                            <span className="text-cream/50">Memory ({memLabel})</span>
                            <span className="text-cream/85">{usd(memPerSec * hours * 3600)}</span>
                        </div>
                        <div className="flex items-center justify-between py-2">
                            <span className="text-cream/50">Rate</span>
                            <span className="text-cream/85">{usd(perSecond, 7)}/s</span>
                        </div>
                    </div>

                    <p className="mt-auto pt-8 text-xs leading-relaxed text-cream/50">
                        Estimate only. Or bring your own compute and run your actors and applications on AWS, Vercel,
                        Railway, or bare metal, paid directly to your provider.
                    </p>
                </InkPanel>
            </div>
        </div>
    );
};

interface Plan {
    name: string;
    prefix?: string;
    price: string;
    period: string;
    desc: string;
    features: string[];
    cta: string;
    highlight: boolean;
    inkHeader?: boolean;
}

const Pricing = () => {
    const cloudPlans: Plan[] = [
        {
            name: "Free",
            price: "$0",
            period: "/mo",
            desc: "For prototyping and small projects.",
            features: [
                "100,000 Awake Actor Hours /mo limit",
                "$5 /mo Compute limit",
                "1 vCPU Max",
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
                "Up to 8 vCPU",
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
                "Up to 8 vCPU",
                "25 Billion Reads /mo included",
                "50 Million Writes /mo included",
                "5GB Storage included",
                "1TB Egress included",
                "MFA",
                "Slack Support"
            ],
            cta: "Get Started",
            highlight: false
        }
    ];

    // Enterprise Edition is the self-hosted, on-prem offering. It is the only
    // enterprise tier, so it is shown on both toggle states: alongside the
    // self-hosted plans, and appended to the cloud plans so it stays visible by
    // default (it keeps its "Self-Hosted" header tag in either view).
    const enterpriseEditionPlan: Plan = {
        name: "Enterprise Edition",
        price: "Custom",
        period: "",
        desc: "Closed-source, production-grade features to run Rivet at enterprise scale in your own VPC, customer environments, or regulated networks.",
        features: [
            "Actor orchestration engine",
            "FoundationDB persistence layer",
            "Cloud layer for multi-tenant",
            "SQLite backup",
            "SQLite PITR",
            "Forking",
            "ACL system",
            "ACL for agents",
            "Advanced ClickHouse analytics",
            "OpenTelemetry integration",
            "Alert manager rules, Prometheus rules, Grafana configs",
            "Kubernetes manifests",
            "Air-gapped & sovereign-cloud deployments",
            "Priority support & SLA",
            "Hardening guidance for FedRAMP, HIPAA, regulated industries"
        ],
        cta: "Contact Sales",
        highlight: false,
        inkHeader: true
    };

    const plans = [...cloudPlans, enterpriseEditionPlan];

    const usagePricing: { resource: string; price: string; unit: string; prefix?: string }[] = [
        { resource: "Awake Actors", price: "$0.05", unit: "per 1k Awake Actor Hours" },
        { resource: "State Storage", price: "$0.40", unit: "per GB-month" },
        { resource: "Reads*", price: "$0.20", unit: "per million reads" },
        { resource: "Writes*", price: "$1", unit: "per million writes" },
        { resource: "Egress", price: "$0.15", unit: "per GB" },
        { resource: "Compute", prefix: "From", price: "$0.0000330", unit: "per vCPU-second + $0.0000029/GiB-s" },
    ];

    return (
        <section id="pricing" className="pb-16 pt-32 md:pb-32 md:pt-40">
            <div className="mx-auto max-w-7xl px-6">
                <div className="flex flex-col gap-12">
                    <div className="flex flex-col items-center text-center">
                        <h2 className={`mb-2 ${SECTION_H2_CLASS}`}>Simple, predictable pricing</h2>
                        <p className="mt-2 max-w-xl text-base leading-relaxed text-ink-soft">
                            Pay for coordination, state, and compute. Run your actors and applications on Rivet Compute, or bring your own and run them anywhere.
                        </p>
                    </div>

                    <div className="flex flex-col gap-12">
                            <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-4">
                                {plans.map((plan, idx) => (
                                    <div
                                       key={idx}
                                       className={`flex flex-col border bg-white/55 ${
                                           plan.highlight ? 'border-pine/60' : 'border-ink/10'
                                       }`}
                                    >
                                        {plan.inkHeader ? (
                                            <div className="selection-paper flex items-center justify-between gap-4 bg-ink px-7 py-3">
                                                <span className="text-sm font-medium text-cream">{plan.name}</span>
                                                <span className="font-mono text-[11px] uppercase tracking-[0.16em] text-cream/60">On-Prem</span>
                                            </div>
                                        ) : null}
                                        <div className="flex flex-grow flex-col p-7">
                                            {!plan.inkHeader ? (
                                                <h3 className="mb-2 text-lg font-medium text-ink">{plan.name}</h3>
                                            ) : null}

                                            <div className="mb-6">
                                                {plan.prefix && <span className="mb-1 block font-mono text-[11px] uppercase tracking-[0.16em] text-ink-faint">{plan.prefix}</span>}
                                                <div className="flex items-baseline gap-1">
                                                    <span className="text-3xl font-medium tracking-[-0.015em] text-ink">{plan.price}</span>
                                                    {plan.period && <span className="ml-1 font-mono text-xs text-ink-faint">{plan.period}</span>}
                                                </div>
                                            </div>

                                            <div className="mb-6 h-px bg-ink/10" />

                                            {plan.desc && <p className="mb-6 min-h-[2.5rem] text-sm leading-relaxed text-ink-soft">{plan.desc}</p>}

                                            <div className="mb-8 space-y-3">
                                                {plan.features.map((feat, i) => (
                                                    <div key={i} className="flex items-start gap-3 text-xs text-ink-soft">
                                                        <Check className="mt-0.5 h-3 w-3 flex-shrink-0 text-pine" />
                                                        <span>{feat}</span>
                                                    </div>
                                                ))}
                                            </div>

                                            <a href={plan.cta === "Contact Sales" ? "/sales" : "https://dashboard.rivet.dev"}
                                                className={`mt-auto w-full rounded-md py-3 text-center text-sm font-medium transition-colors ${
                                                    plan.highlight
                                                    ? 'bg-accent-deep text-white hover:bg-accent'
                                                    : 'border border-ink/20 text-ink-soft hover:border-ink/40 hover:text-ink'
                                                }`}
                                            >
                                                {plan.cta}
                                            </a>
                                        </div>
                                    </div>
                                ))}
                            </div>

                            {/* YC & a16z Speedrun Callout */}
                            <div className="rounded-lg border border-ink/10 bg-white/55 p-6">
                                    <div className="flex flex-col gap-6 md:flex-row md:items-center md:justify-between">
                                        <div>
                                            <p className="mb-2 text-base font-medium text-ink">Startup Deal: 50% off for 12 months</p>
                                            <div className="flex flex-wrap items-center gap-2 text-sm text-ink-soft">
                                                <span>For</span>
                                                <div className="flex items-center gap-2 rounded-full border border-ink/15 bg-white/55 px-3 py-1.5 text-xs text-ink-soft">
                                                    <img src={imgYC.src} alt="Y Combinator" className="h-4 w-auto" />
                                                    <span>Y Combinator</span>
                                                </div>
                                                <span>and</span>
                                                <div className="flex items-center gap-2 rounded-full border border-ink/15 bg-white/55 px-3 py-1.5 text-xs text-ink-soft">
                                                    <img src={imgA16z.src} alt="a16z" className="h-3 w-auto invert" />
                                                    <span>a16z Speedrun</span>
                                                </div>
                                                <span>companies</span>
                                            </div>
                                        </div>
                                        <a
                                            href="/startups"
                                            className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-ink/20 px-4 py-2 text-sm text-ink-soft transition-colors hover:border-ink/40 hover:text-ink"
                                        >
                                            Claim the deal
                                            <ArrowRight className="h-4 w-4" />
                                        </a>
                                    </div>
                                </div>

                            {/* Usage pricing, compute calculator, and plan comparison */}
                                <>
                                    {/* Usage Pricing Section */}
                                    <div className="border-t border-ink/10 pt-16">
                                        <h3 className="mb-3 text-2xl font-medium tracking-[-0.015em] text-ink">Usage Pricing</h3>
                                        <p className="mb-8 text-base leading-relaxed text-ink-soft">Metered costs for scaling beyond plan limits.</p>

                                        <div className="grid grid-cols-2 gap-6 md:grid-cols-3">
                                            {usagePricing.map((item, i) => (
                                                <div key={i} className="border-t border-ink/10 pt-6">
                                                    <div className="mb-2 font-mono text-[11px] uppercase tracking-[0.16em] text-ink-faint">{item.resource}</div>
                                                    {item.prefix && <span className="mb-1 block font-mono text-[10px] uppercase tracking-[0.16em] text-ink-faint">{item.prefix}</span>}
                                                    <div className="mb-1 font-mono text-2xl text-ink">{item.price}</div>
                                                    <div className="text-xs text-ink-faint">{item.unit}</div>
                                                </div>
                                            ))}
                                        </div>
                                        <p className="mt-6 text-xs text-ink-faint">* Reads and writes to persisted actor state, not in-memory operations within an actor</p>
                                        <p className="mt-2 text-xs text-ink-faint">Compute runs on Rivet Compute, billed per active second. Or bring your own compute and run your actors and applications on AWS, Vercel, Railway, or bare metal, paid directly to your provider.</p>
                                    </div>

                                    <ComputeCalculator />

                                    <ComparisonTable />
                                </>
                    </div>
                </div>
            </div>
        </section>
    );
};


export default function PricingPageClient() {
  return (
    <div className="paper-grain min-h-screen font-sans text-ink-soft">
      <main>
        <Pricing />
        <SelfHostingComparison />
      </main>
    </div>
  );
}
