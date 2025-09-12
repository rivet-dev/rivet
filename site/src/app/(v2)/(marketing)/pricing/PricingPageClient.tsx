"use client";
import { CheckIcon, MinusIcon, PlusIcon } from "@heroicons/react/16/solid";
import { MobilePricingTabs } from "./components/MobilePricingTabs";
import { useState } from "react";
import clsx from "clsx";

const tiers = [
	{
		name: "Community",
		priceMonthly: "$0",
		href: "https://hub.rivet.gg/",
		highlights: [
			{ description: "$5 Free credits", icon: "gift" },
			{ description: "Community Support", icon: "check" },
		],
	},
	{
		name: "Pro",
		priceMonthly: "$20",
		href: "https://hub.rivet.gg/",
		highlights: [
			{ description: "$20 Free credits /mo", icon: "gift" },
			{ description: "Everything in Community", icon: "check" },
			{ description: "No Usage Limits", icon: "check" },
			{ description: "Unlimited Seats", icon: "check" },
			{ description: "Email Support", icon: "check" },
		],
	},
	{
		name: "Team",
		priceMonthly: "$200",
		href: "https://hub.rivet.gg/",
		highlights: [
			{ description: "$200 Free credits /mo", icon: "gift" },
			{ description: "Everything in Pro", icon: "check" },
			{ description: "Dedicated Hardware", icon: "check" },
			{ description: "Custom Regions", icon: "check" },
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
			{
				description: "Custom Storage Reads, Writes, and Stored Data",
				icon: "check",
			},
			{ description: "Custom Log Retention", icon: "check" },
		],
	},
];

const sections = [
	{
		name: "Support",
		features: [
			{
				name: "Support",
				tiers: {
					Community: "Community Support",
					Pro: "Email",
					Team: "Slack & Email",
					Enterprise: "Slack & Email",
				},
			},
		],
	},
	{
		name: "Infrastructure Credits",
		features: [
			{
				name: "Credits",
				tiers: {
					Community: "$5.00 Credit",
					Pro: "$20.00 Credits /Mo",
					Team: "$200.00 Credits /Mo",
					Enterprise: "Custom",
				},
			},
		],
	},
	{
		name: "Seat Limits",
		features: [
			{
				name: "Seat Limits",
				tiers: {
					Community: "2 Seats",
					Pro: "Unlimited",
					Team: "Unlimited",
					Enterprise: "Unlimited",
				},
			},
		],
	},
	{
		name: "Usage Limit",
		features: [
			{
				name: "Usage Limit",
				tiers: {
					Community: "Limited",
					Pro: "Unlimited",
					Team: "Unlimited",
					Enterprise: "Unlimited",
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
							<div className="mt-8 flex flex-col items-start gap-0 min-h-[120px]">
								{(tier.name === "Pro" ||
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
								{(tier.name === "Pro" ||
									tier.name === "Team") && (
									<div className="text-sm text-white/40 mt-1">
										+ Usage
									</div>
								)}
							</div>
							<div className="mt-8 border-t border-white/10 pt-4">
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
			<div className="relative isolate overflow-hidden pb-8 sm:pb-10 pt-40">
				<div className="mx-auto max-w-[1200px] px-6 lg:px-8">
					<div className="text-center">
						<h1 className="text-6xl font-700 text-white leading-[1.1] tracking-normal">
							Rivet {activeTab === "cloud" ? "Cloud Pricing" : "Self-Host"}
						</h1>
						<p className="mt-6 max-w-2xl mx-auto text-xl leading-[1.2] tracking-tight font-500 text-white/60">
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
						<div className="flex flex-col items-center mt-16">
							<a
								href="/docs/cloud/pricing"
								target="_self"
								rel="noopener noreferrer"
								className="inline-flex items-center justify-center px-6 py-3 border border-white/20 text-white font-medium rounded-lg hover:bg-white/10 transition-colors"
							>
								View Usage Pricing
							</a>
						</div>
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
