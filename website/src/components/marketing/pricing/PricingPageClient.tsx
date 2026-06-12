"use client";

import { useState } from 'react';
import {
  ArrowRight,
  Check,
  Server,
  Cloud
} from 'lucide-react';
import rivetLogoWhite from '@/images/rivet-logos/icon-white.svg';
import imgYC from '@/images/logos/yc.svg';
import imgA16z from '@/images/logos/a16z.svg';
import { SECTION_H2_CLASS, SUBTITLE_CLASS } from '@/components/marketing/typography';
import { Eyebrow } from '@/components/marketing/editorial/Eyebrow';
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

  return (
    <section className="border-t border-ink/10 py-16 md:py-32">
      <div className="mx-auto max-w-7xl px-6">
        <div className="flex flex-col gap-12">
          <div className="max-w-xl">
            <Eyebrow index="04" label="Deployment models" className="mb-4" />
            <h2 className={`mb-2 ${SECTION_H2_CLASS}`}>Compare Deployment Models</h2>
            <p className={SUBTITLE_CLASS}>
                Rivet is open source. Use Rivet Cloud for managed infrastructure, or self-host in your VPC, your customers' environments, or air-gapped networks.
            </p>
          </div>

          <div className="grid gap-8 md:grid-cols-2">
              {/* Rivet Cloud Card */}
              <div className="flex flex-col border border-ink/10 bg-white/55 p-7">
                  <div className="mb-6 flex items-center gap-3">
                       <img src={rivetLogoWhite.src} alt="Rivet" className="h-9 w-9 invert" />
                       <h3 className="text-lg font-medium text-ink">Rivet Cloud</h3>
                  </div>
                  <p className="mb-8 text-sm leading-relaxed text-ink-soft">
                      Managed cloud solution for personal projects to enterprise.
                  </p>

                  <div className="flex-grow font-mono text-sm">
                      {cloudSpecs.map(({ label, value }) => (
                          <div key={label} className="flex items-center justify-between gap-4 border-b border-ink/10 py-3.5">
                              <span className="text-[11px] uppercase tracking-[0.16em] text-ink-faint">{label}</span>
                              <span className="text-right text-pine">{value}</span>
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
              <InkPanel
                  caption="Fig. 01 — Self-hosted spec sheet"
                  className="flex flex-col [&>div:first-child]:flex-grow"
              >
                  <div className="flex h-full flex-col p-7">
                      <div className="mb-6 flex items-center gap-3">
                           <Server className="h-6 w-6 text-sage" />
                           <h3 className="text-lg font-medium text-cream">Self-Hosted</h3>
                      </div>
                      <p className="mb-8 text-sm leading-relaxed text-cream/60">
                          Maximum control for air-gapped environments and regulated workloads. Deploy inside the boundary your existing controls already cover.
                      </p>

                      <div className="flex-grow font-mono text-sm">
                          {selfHostedSpecs.map(({ label, value }) => (
                              <div key={label} className="flex items-center justify-between gap-4 border-b border-cream/10 py-3.5">
                                  <span className="text-[11px] uppercase tracking-[0.16em] text-cream/50">{label}</span>
                                  <span className="text-right text-sage">{value}</span>
                              </div>
                          ))}
                      </div>

                      <a href="https://github.com/rivet-dev/rivet"
                          className="mt-8 w-full rounded-md border border-cream/20 py-3 text-center text-sm text-cream/85 transition-colors hover:border-cream/40 hover:text-cream"
                      >
                          View on GitHub
                      </a>
                      <p className="mt-4 text-center text-xs text-cream/50">
                          High-touch deployment? <a href="/enterprise" className="text-sage transition-colors hover:text-cream">Rivet for Enterprise</a>
                      </p>
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
          <div className="flex justify-center"><Check className="h-4 w-4 text-pine" /></div> :
          <div className="flex justify-center"><div className="h-1.5 w-1.5 rounded-full bg-ink/20" /></div>;
      }
      return <span className="text-sm text-ink-soft">{value}</span>;
    };

    return (
        <div className="mt-24 border-t border-ink/10 pt-16">
            <Eyebrow index="03" label="Plan comparison" className="mb-4" />
            <h3 className="mb-12 text-2xl font-medium tracking-[-0.015em] text-ink">Compare Plans</h3>
            <div className="overflow-x-auto">
                <table className="w-full min-w-[800px] border-collapse">
                    <thead>
                        <tr className="border-b border-ink/15">
                            <th className="w-1/4 p-4 text-left font-mono text-[11px] font-medium uppercase tracking-[0.16em] text-ink-faint">Feature</th>
                            <th className="w-[18%] p-4 text-center text-sm font-medium text-ink">Free</th>
                            <th className="w-[18%] p-4 text-center text-sm font-medium text-pine">Hobby</th>
                            <th className="w-[18%] p-4 text-center text-sm font-medium text-ink">Team</th>
                            <th className="w-[18%] p-4 text-center text-sm font-medium text-ink">Enterprise</th>
                        </tr>
                    </thead>
                    <tbody>
                        {features.map((feature, i) => (
                            <tr key={i} className="border-b border-ink/10">
                                <td className="p-4 text-sm text-ink-soft">{feature.name}</td>
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
    const [isCloud, setIsCloud] = useState(true);

    const cloudPlans: Plan[] = [
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

    const selfHostedPlans: Plan[] = [
        {
            name: "Open Source",
            price: "Free",
            period: "Forever",
            desc: "Rivet is open source. Run it on your own infrastructure with no usage limits.",
            features: [
                "Single Rust binary or Docker image",
                "Air-gapped & on-prem deployments",
                "BYO database (Postgres or filesystem)",
                "Apache 2.0 license, full source access",
                "Community support"
            ],
            cta: "Get Started",
            highlight: false
        },
        {
            name: "Enterprise Edition",
            price: "Custom",
            period: "",
            desc: "Production self-host bundle for teams running Rivet inside their own VPC, customer environments, or regulated networks.",
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
        }
    ];

    const plans = isCloud ? cloudPlans : selfHostedPlans;

    const usagePricing = [
        { resource: "Awake Actors", price: "$0.05", unit: "per 1k Awake Actor Hours" },
        { resource: "State Storage", price: "$0.40", unit: "per GB-month" },
        { resource: "Reads*", price: "$0.20", unit: "per million reads" },
        { resource: "Writes*", price: "$1", unit: "per million writes" },
        { resource: "Egress", price: "$0.15", unit: "per GB" },
        { resource: "Compute", price: "BYO", unit: "Paid to your provider" },
    ];

    return (
        <section id="pricing" className="pb-16 pt-32 md:pb-32 md:pt-40">
            <div className="mx-auto max-w-7xl px-6">
                <div className="flex flex-col gap-12">
                    <div className="flex flex-col items-center text-center">
                        <Eyebrow index="01" label="Plans & pricing" className="mb-4" />
                        <h2 className={`mb-2 ${SECTION_H2_CLASS}`}>
                            {isCloud ? "Simple, predictable pricing" : "Run it where your data lives"}
                        </h2>
                        <p className="mb-6 mt-2 max-w-xl text-base leading-relaxed text-ink-soft">
                            {isCloud
                                ? "Pay for coordination and state. Compute costs are billed directly by your chosen cloud provider."
                                : "Deploy Rivet inside your VPC, your customer's environment, or fully air-gapped. Use the compliance posture you already have."
                            }
                        </p>

                        {/* On-prem callout — visible whichever tier is selected */}
                        <button
                            onClick={() => setIsCloud(false)}
                            className="mb-6 inline-flex items-center gap-2 rounded-full border border-pine/30 px-3 py-1.5 text-xs text-pine transition-colors hover:border-pine/60"
                        >
                            <Server className="h-3 w-3" />
                            Need on-prem, air-gapped, or in your customer's VPC?
                            <span className="font-medium">Self-host →</span>
                        </button>

                        {/* Toggle */}
                        <div className="inline-flex rounded-lg border border-ink/15 p-1">
                            <button
                                onClick={() => setIsCloud(true)}
                                className={`flex items-center gap-2 rounded-md px-4 py-2 text-sm transition-all ${isCloud ? 'bg-ink text-cream' : 'text-ink-soft hover:text-ink'}`}
                            >
                                <Cloud className="h-4 w-4" /> Cloud
                            </button>
                            <button
                                onClick={() => setIsCloud(false)}
                                className={`flex items-center gap-2 rounded-md px-4 py-2 text-sm transition-all ${!isCloud ? 'bg-ink text-cream' : 'text-ink-soft hover:text-ink'}`}
                            >
                                <Server className="h-4 w-4" /> Self-Hosted
                            </button>
                        </div>
                    </div>

                    <div className={`grid gap-6 ${isCloud ? 'md:grid-cols-2 lg:grid-cols-4' : 'md:grid-cols-2 max-w-4xl mx-auto'}`}>
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
                                        <span className="font-mono text-[11px] uppercase tracking-[0.16em] text-sage">Self-Hosted</span>
                                    </div>
                                ) : null}
                                <div className="flex flex-grow flex-col p-7">
                                    {plan.highlight ? <Eyebrow label="Recommended" className="mb-3" /> : null}
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

                                    <a href={plan.cta === "Contact Sales" || plan.name === "Enterprise" ? "/sales" : "https://dashboard.rivet.dev"}
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
                    {isCloud && (
                        <div className="mt-12 rounded-lg border border-ink/10 bg-cream p-6">
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
                    )}

                    {/* Only show usage and comparison for Cloud */}
                    {isCloud && (
                        <>
                            {/* Usage Pricing Section */}
                            <div className="mt-12 border-t border-ink/10 pt-16">
                                <Eyebrow index="02" label="Metered usage" className="mb-4" />
                                <h3 className="mb-3 text-2xl font-medium tracking-[-0.015em] text-ink">Usage Pricing</h3>
                                <p className="mb-8 text-base leading-relaxed text-ink-soft">Metered costs for scaling beyond plan limits.</p>

                                <div className="grid grid-cols-2 gap-6 md:grid-cols-3">
                                    {usagePricing.map((item, i) => (
                                        <div key={i} className="border-t border-ink/10 pt-6">
                                            <div className="mb-2 font-mono text-[11px] uppercase tracking-[0.16em] text-ink-faint">{item.resource}</div>
                                            <div className={`mb-1 font-mono text-2xl ${item.price === "BYO" ? "text-ink-faint" : "text-ink"}`}>{item.price}</div>
                                            <div className="text-xs text-ink-faint">{item.unit}</div>
                                        </div>
                                    ))}
                                </div>
                                <p className="mt-6 text-xs text-ink-faint">* Reads and writes to persisted actor state, not in-memory operations within an actor</p>
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
    <div className="paper-grain min-h-screen font-sans text-ink-soft">
      <main>
        <Pricing />
        <SelfHostingComparison />
      </main>
    </div>
  );
}
