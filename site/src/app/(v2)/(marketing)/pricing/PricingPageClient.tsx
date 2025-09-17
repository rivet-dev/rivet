"use client";
import { CheckIcon, MinusIcon, PlusIcon } from "@heroicons/react/16/solid";
import { MobilePricingTabs } from "./components/MobilePricingTabs";
import { useState } from "react";
import clsx from "clsx";
import Link from "next/link";
import { Icon, faArrowRight } from "@rivet-gg/icons";

const tiers = [
	{
		name: "Free",
		priceMonthly: "$0",
		href: "https://hub.rivet.gg/",
		highlights: [
			{ description: "5GB Limit", icon: "check" },
			{ description: "5 Million Writes /mo Limit", icon: "check" },
			{ description: "200 Million Reads /mo Limit", icon: "check" },
			{ description: "100GB Egress Limit", icon: "check" },
			{ description: "Community Support", icon: "check" },
		],
	},
		{
			name: "Hobby",
			priceMonthly: "$5",
			href: "https://hub.rivet.gg/",
		highlights: [
			{ description: "25 Billion Reads /mo included", icon: "gift" },
			{ description: "50 Million Writes /mo included", icon: "gift" },
			{ description: "5GB Storage included", icon: "gift" },
			{ description: "1TB Egress included", icon: "gift" },
			{ description: "Email Support", icon: "check" },
		],
	},
	{
		name: "Team",
		priceMonthly: "$200",
		href: "https://hub.rivet.gg/",
		highlights: [
			{ description: "25 Billion Reads /mo included", icon: "gift" },
			{ description: "50 Million Writes /mo included", icon: "gift" },
			{ description: "5GB Storage included", icon: "gift" },
			{ description: "1TB Egress included", icon: "gift" },
			{ description: "MFA", icon: "check" },
			{ description: "Slack Support", icon: "check" },
		],
	},
		{
			name: "Enterprise",
			priceMonthly: "Custom",
			href: "/sales",
			highlights: [
				{ description: "Everything in Team", icon: "check" },
				{ description: "Priority Support", icon: "check" },
				{ description: "SLA", icon: "check" },
				{ description: "OIDC SSO provider", icon: "check" },
				{ description: "On-Prem Deployment", icon: "check" },
				{ description: "Audit Logs", icon: "check" },
				{ description: "Custom Roles", icon: "check" },
				{ description: "Device Tracking", icon: "check" },
				{ description: "Volume Pricing", icon: "check" },
			],
		},
];

const sections = [
	{
		name: "Usage Included",
		features: [
			{
				name: "Storage",
				tiers: {
					Free: "5GB",
					Hobby: "5GB included",
					Team: "5GB included",
					Enterprise: "Custom",
				},
			},
			{
				name: "Reads per month",
				tiers: {
					Free: "200 Million",
					Hobby: "25 Billion included",
					Team: "25 Billion included", 
					Enterprise: "Custom",
				},
			},
			{
				name: "Writes per month",
				tiers: {
					Free: "5 Million",
					Hobby: "50 Million included",
					Team: "50 Million included",
					Enterprise: "Custom",
				},
			},
			{
				name: "Egress",
				tiers: {
					Free: "100GB Limit",
					Hobby: "1TB included",
					Team: "1TB included",
					Enterprise: "Custom",
				},
			},
		],
	},
	{
		name: "Support",
		features: [
			{
				name: "Support",
				tiers: {
					Free: "Community Support",
					Hobby: "Email",
					Team: "Slack & Email",
					Enterprise: "Slack & Email",
				},
			},
		],
	},
	{
		name: "Security & Enterprise",
		features: [
			{
				name: "MFA",
				tiers: {
					Free: false,
					Hobby: false,
					Team: true,
					Enterprise: true,
				},
			},
			{
				name: "Custom Regions",
				tiers: {
					Free: false,
					Hobby: false,
					Team: false,
					Enterprise: true,
				},
			},
			{
				name: "SLA",
				tiers: {
					Free: false,
					Hobby: false,
					Team: false,
					Enterprise: true,
				},
			},
			{
				name: "Audit Logs",
				tiers: {
					Free: false,
					Hobby: false,
					Team: false,
					Enterprise: true,
				},
			},
			{
				name: "Custom Roles",
				tiers: {
					Free: false,
					Hobby: false,
					Team: false,
					Enterprise: true,
				},
			},
			{
				name: "Device Tracking",
				tiers: {
					Free: false,
					Hobby: false,
					Team: false,
					Enterprise: true,
				},
			},
			{
				name: "Volume Pricing",
				tiers: {
					Free: false,
					Hobby: false,
					Team: false,
					Enterprise: true,
				},
			},
		],
	},
];

function PricingTable() {
	return (
		<table className="w-full text-left max-sm:hidden">
			<caption className="sr-only">Pricing plan comparison</caption>
			<colgroup>
				<col className="w-2/5" />
				<col className="w-1/5" />
				<col className="w-1/5" />
				<col className="w-1/5" />
			</colgroup>
			<thead>
				<tr>
					<th
						className="px-0 py-6 text-left text-2xl font-normal text-white"
						colSpan={1}
					>
						Compare Rivet Cloud Plans
					</th>
					{tiers.slice(1).map((_, idx) => (
						<td key={idx} />
					))}
				</tr>
				<tr>
					<th className="p-0" />
					{tiers.map((tier) => (
						<th key={tier.name} scope="col" className="p-0">
							<div className="text-lg font-normal text-white">
								{tier.name}{" "}
								<span className="sr-only">plan</span>
							</div>
						</th>
					))}
				</tr>
			</thead>
			{sections.map((section) => (
				<tbody key={section.name} className="group">
					{section.features.map((feature) => (
						<tr
							key={feature.name}
							className="border-b border-white/5 last:border-none"
						>
							<th
								scope="row"
								className="px-0 py-4 text-sm font-normal text-white/40 text-left align-middle"
							>
								{feature.name}
							</th>
							{tiers.map((tier) => (
								<td
									key={tier.name}
									className="p-4 max-sm:text-center"
								>
									{typeof feature.tiers[tier.name] ===
									"string" ? (
										feature.name === "Regions" ? (
											<a
												href="https://rivet.gg/docs/general/edge"
												target="_blank"
												rel="noopener noreferrer"
												className="text-sm text-white underline hover:brightness-125"
											>
												<span className="sr-only">
													Learn more about regions:{" "}
												</span>
												{feature.tiers[tier.name]}
											</a>
										) : (
											<>
												<span className="sr-only">
													{tier.name} includes:
												</span>
												<span className="text-sm text-white">
													{feature.tiers[tier.name]}
												</span>
											</>
										)
									) : (
										<>
											{feature.tiers[tier.name] ===
											true ? (
												<CheckIcon
													aria-hidden="true"
													className="inline-block size-4 fill-green-400"
												/>
											) : (
												<MinusIcon
													aria-hidden="true"
													className="inline-block size-4 fill-white/10"
												/>
											)}

											<span className="sr-only">
												{feature.tiers[tier.name] ===
												true
													? `Included in ${tier.name}`
													: `Not included in ${tier.name}`}
											</span>
										</>
									)}
								</td>
							))}
						</tr>
					))}
				</tbody>
			))}
		</table>
	);
}

function PricingTiers() {
	return (
		<div className="grid grid-cols-1 gap-8 md:grid-cols-2 lg:grid-cols-4 justify-items-center items-stretch">
			{tiers.map((tier) => (
				<div
					key={tier.name}
					className="block group w-72 max-w-xs h-full flex flex-col"
				>
					<div className="rounded-xl bg-[#121212] group-hover:bg-zinc-800/90 border border-white/5 group-hover:border-[white]/20 shadow-sm transition-all duration-200 relative overflow-hidden flex-1 flex flex-col min-h-[700px]">
						<div className="p-8 pb-7 flex-1 flex flex-col">
							<h2 className="text-lg font-normal text-white">
								{tier.name}{" "}
								<span className="sr-only">plan</span>
							</h2>
							<div className="mt-8 flex flex-col items-start gap-0 h-[120px]">
								{(tier.name === "Hobby" ||
									tier.name === "Team") && (
									<div className="text-xs text-white/40 mb-1">
										From
									</div>
								)}
								<div className="flex items-baseline gap-1">
									<div className="text-5xl font-semibold text-white">
										{tier.priceMonthly}
									</div>
									{tier.name !== "Enterprise" && (
										<span className="text-base text-white/40">
											/mo
										</span>
									)}
								</div>
								{(tier.name === "Hobby" ||
									tier.name === "Team") && (
									<div className="text-sm text-white/40 mt-1">
										+ Usage
									</div>
								)}
							</div>
							<div className="mt-6 border-t border-white/10 pt-4">
								<h3 className="text-sm font-normal text-white">
									Includes:
								</h3>
								<ul className="mt-3 space-y-3">
									{tier.highlights.map((highlight, idx) => (
										<li
											key={highlight.description}
											className={`group flex items-start gap-4 text-sm text-white/40 ${highlight.icon === "gift" ? "text-green-400" : ""}`}
										>
											{highlight.icon === "gift" ? (
												<span className="inline-flex h-6 items-center">
													<PlusIcon
														aria-hidden="true"
														className="size-4 fill-green-400"
													/>
												</span>
											) : (
												<span className="inline-flex h-6 items-center">
													<CheckIcon
														aria-hidden="true"
														className="size-4 fill-white/40"
													/>
												</span>
											)}
											{highlight.description}
										</li>
									))}
								</ul>
							</div>
							<div className="mt-auto">
								<a
									href={tier.href}
									aria-label={`Start a free trial on the ${tier.name} plan`}
									className="inline-flex items-center justify-center px-3.5 py-2 text-base font-medium rounded-xl transition-all duration-200 active:scale-[0.97] bg-[#FF5C00]/90 hover:bg-[#FF5C00] hover:brightness-110 text-white"
								>
									{tier.name === "Enterprise"
										? "Contact"
										: "Get Started"}
								</a>
							</div>
						</div>
					</div>
				</div>
			))}
		</div>
	);
}

function UsagePricingTable() {
	return (
		<div className="mt-16">
			<div className="text-center mb-8">
				<h2 className="text-3xl font-600 text-white mb-4">
					Usage Pricing
				</h2>
				<p className="text-lg text-white/60 max-w-2xl mx-auto">
					Pay only for what you use beyond the included allowances
				</p>
			</div>
			
			<div className="max-w-2xl mx-auto">
				<div className="bg-white/5 border border-white/10 rounded-lg overflow-hidden">
					<table className="w-full">
						<thead>
							<tr className="border-b border-white/10">
								<th className="px-6 py-4 text-left text-sm font-medium text-white">
									Resource
								</th>
								<th className="px-6 py-4 text-right text-sm font-medium text-white">
									Price
								</th>
							</tr>
						</thead>
						<tbody>
							<tr className="border-b border-white/5">
								<td className="px-6 py-4 text-sm text-white/80">
									Storage
								</td>
								<td className="px-6 py-4 text-sm text-white text-right">
									$0.40 per GB-month
								</td>
							</tr>
							<tr className="border-b border-white/5">
								<td className="px-6 py-4 text-sm text-white/80">
									Reads
								</td>
								<td className="px-6 py-4 text-sm text-white text-right">
									$1.00 per billion reads
								</td>
							</tr>
									<tr className="border-b border-white/5">
										<td className="px-6 py-4 text-sm text-white/80">
											Writes
										</td>
										<td className="px-6 py-4 text-sm text-white text-right">
											$1.00 per million writes
										</td>
									</tr>
									<tr>
										<td className="px-6 py-4 text-sm text-white/80">
											Egress
										</td>
										<td className="px-6 py-4 text-sm text-white text-right">
											$0.15 per GB
										</td>
									</tr>
						</tbody>
					</table>
				</div>
			</div>
		</div>
	);
}

interface ToggleProps {
	options: { value: string; label: string }[];
	activeValue: string;
	onChange: (value: string) => void;
}

function Toggle({ options, activeValue, onChange }: ToggleProps) {
	return (
		<div className="flex items-center justify-center">
			<div className="inline-flex items-center bg-white/5 border border-white/10 rounded-lg p-1">
				{options.map((option) => (
					<button
						key={option.value}
						onClick={() => onChange(option.value)}
						className={clsx(
							"px-4 py-2 text-sm font-medium rounded-md transition-all duration-200 whitespace-nowrap",
							activeValue === option.value
								? "bg-white/10 text-white border border-white/20"
								: "text-white/60 hover:text-white/80 hover:bg-white/5"
						)}
					>
						{option.label}
					</button>
				))}
			</div>
		</div>
	);
}

export default function PricingPageClient() {
	const [activeTab, setActiveTab] = useState("cloud");

	const toggleOptions = [
		{ value: "cloud", label: "Cloud" },
		{ value: "selfhost", label: "Self-Host" },
	];

	return (
		<main className="min-h-screen w-full max-w-[1500px] mx-auto md:px-8">
			<div className="relative isolate overflow-hidden pb-8 sm:pb-10 pt-48">
				<div className="mx-auto max-w-[1200px] px-6 lg:px-8">
					<div className="text-center">
						<h1 className="text-6xl font-normal text-white leading-[1.1] tracking-normal">
							Rivet {activeTab === "cloud" ? "Cloud Pricing" : "Self-Host"}
						</h1>
						<p className="mt-6 max-w-2xl mx-auto text-xl leading-[1.2] tracking-tight font-extralight text-white/60">
							{activeTab === "cloud" 
								? "Start with free and scale as you grow."
								: "Deploy Rivet on your own infrastructure."
							}
						</p>
						<div className="mt-8">
							<Toggle
								options={toggleOptions}
								activeValue={activeTab}
								onChange={setActiveTab}
							/>
						</div>
					</div>
				</div>
			</div>

			{activeTab === "cloud" ? (
				<>
					<div className="relative mx-auto max-w-2xl px-6 lg:max-w-7xl lg:px-8 pt-16 sm:pt-24">
						<PricingTiers />
						<UsagePricingTable />
					</div>

					<div className="mx-auto max-w-2xl px-6 pt-16 sm:pt-24 lg:max-w-7xl lg:px-8 pb-24">
						<PricingTable />
						<MobilePricingTabs />
					</div>
				</>
			) : (
				<div className="relative mx-auto max-w-2xl px-6 lg:max-w-7xl lg:px-8 pt-16 sm:pt-24 pb-24">
					<div className="text-center">
						<h2 className="text-3xl font-600 text-white mb-8">
							Self-Hosted Rivet
						</h2>
						<p className="text-lg text-white/60 mb-12 max-w-2xl mx-auto">
							Deploy Rivet on your own infrastructure with full control and no usage limits.
						</p>
						
						<div className="grid grid-cols-1 md:grid-cols-2 gap-8 max-w-4xl mx-auto">
							<div className="bg-white/5 border border-white/10 rounded-lg p-6 flex flex-col">
								<h3 className="text-xl font-600 text-white mb-4">Open Source</h3>
								<p className="text-white/60 mb-4">
									Rivet is open source and free to use on your own infrastructure.
								</p>
								<ul className="space-y-2 text-white/80 mb-6">
									<li className="flex items-center gap-2">
										<CheckIcon className="w-4 h-4 text-green-400" />
										No usage limits
									</li>
									<li className="flex items-center gap-2">
										<CheckIcon className="w-4 h-4 text-green-400" />
										Full source code access
									</li>
									<li className="flex items-center gap-2">
										<CheckIcon className="w-4 h-4 text-green-400" />
										Community support
									</li>
								</ul>
								<div className="mt-auto">
									<a
										href="/docs/self-hosting"
										className="inline-flex items-center justify-center w-full px-6 py-3 border border-white/20 text-white font-medium rounded-lg hover:bg-white/10 transition-colors"
									>
										Get Started
									</a>
								</div>
							</div>
							
							<div className="bg-white/5 border border-white/10 rounded-lg p-6 flex flex-col">
								<h3 className="text-xl font-600 text-white mb-4">Enterprise Support</h3>
								<p className="text-white/60 mb-4">
									Get professional support and additional features for your self-hosted deployment.
								</p>
								<ul className="space-y-2 text-white/80 mb-6">
									<li className="flex items-center gap-2">
										<CheckIcon className="w-4 h-4 text-green-400" />
										Priority support
									</li>
									<li className="flex items-center gap-2">
										<CheckIcon className="w-4 h-4 text-green-400" />
										SLA guarantees
									</li>
									<li className="flex items-center gap-2">
										<CheckIcon className="w-4 h-4 text-green-400" />
										Custom integrations
									</li>
								</ul>
								<div className="mt-auto">
									<a
										href="/sales"
										className="inline-flex items-center justify-center w-full px-6 py-3 bg-white text-black font-medium rounded-lg hover:bg-white/90 transition-colors"
									>
										Contact Sales
									</a>
								</div>
							</div>
						</div>
					</div>
				</div>
			)}
		</main>
	);
}


