"use client";

import { Tab } from "@headlessui/react";
import { CheckIcon, MinusIcon } from "@heroicons/react/16/solid";
import { Fragment } from "react";

const tiers = [
	{
		name: "Free",
		href: "https://hub.rivet.gg/",
	},
	{
		name: "Hobby",
		href: "https://hub.rivet.gg/",
	},
	{
		name: "Team",
		href: "https://hub.rivet.gg/",
	},
	{
		name: "Enterprise",
		href: "/sales",
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

export function MobilePricingTabs() {
	return (
		<div className="sm:hidden [&_.tab]:!text-white/60 [&_.tab[data-selected]]:!text-white [&_.tab[data-selected]]:!border-white [&_.tab]:!border-white/10">
			{/* Usage Pricing Table for Mobile */}
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

			<Tab.Group>
				<Tab.List className="flex">
					{tiers.map((tier) => (
						<Tab
							key={tier.name}
							className="w-1/4 border-b border-white/10 py-4 text-base/8 font-medium text-white/60 data-[selected]:border-white data-[selected]:text-white [&:not([data-focus])]:focus:outline-none"
						>
							{tier.name}
						</Tab>
					))}
				</Tab.List>
				<Tab.Panels>
					{tiers.map((tier) => (
						<Tab.Panel key={tier.name}>
							<a
								href={tier.href}
								className="mt-8 block rounded-xl bg-[#FF5C00]/90 hover:bg-[#FF5C00] hover:brightness-110 text-white px-6 py-3 text-center text-sm font-medium transition-all duration-200 active:scale-[0.97]"
							>
								Get started
							</a>
							{sections.map((section) => (
								<Fragment key={section.name}>
									<div className="-mx-6 mt-10 rounded-lg bg-white/5 px-6 py-3 text-sm/6 font-semibold text-white group-first-of-type:mt-5">
										{section.name}
									</div>
									<dl>
										{section.features.map((feature) => (
											<div
												key={feature.name}
												className="grid grid-cols-2 border-b border-white/5 py-4 last:border-none"
											>
												<dt className="text-sm/6 font-normal text-white/70">
													{feature.name}
												</dt>
												<dd className="text-center">
													{typeof feature.tiers[
														tier.name
													] === "string" ? (
														<span className="text-sm/6 text-white">
															{
																feature.tiers[
																	tier.name
																]
															}
														</span>
													) : (
														<>
															{feature.tiers[
																tier.name
															] === true ? (
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
																{feature.tiers[
																	tier.name
																] === true
																	? "Yes"
																	: "No"}
															</span>
														</>
													)}
												</dd>
											</div>
										))}
									</dl>
								</Fragment>
							))}
						</Tab.Panel>
					))}
				</Tab.Panels>
			</Tab.Group>
		</div>
	);
}
