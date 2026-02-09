"use client";

import {
	Icon,
	faArrowRight,
	faCheck,
	faCloudflare,
	faHourglass,
	faMinus,
	faRivet,
	faXmark,
} from "@rivet-gg/icons";
import { motion } from "framer-motion";
import React from "react";

// Feature Status Component
interface FeatureStatusProps {
	status: "yes" | "no" | "partial" | "coming-soon";
	text: string;
}

const FeatureStatus: React.FC<FeatureStatusProps> = ({ status, text }) => {
	let icon, bgColor, textColor;

	switch (status) {
		case "yes":
			icon = faCheck;
			bgColor = "bg-green-500/20";
			textColor = "text-green-500";
			break;
		case "no":
			icon = faXmark;
			bgColor = "bg-red-500/20";
			textColor = "text-red-500";
			break;
		case "partial":
			icon = faMinus;
			bgColor = "bg-amber-500/20";
			textColor = "text-amber-500";
			break;
		case "coming-soon":
			icon = faHourglass;
			bgColor = "bg-purple-500/20";
			textColor = "text-purple-500";
			break;
		default:
			icon = faCheck;
			bgColor = "bg-green-500/20";
			textColor = "text-green-500";
	}

	return (
		<div className="flex items-start">
			<div
				className={`flex-shrink-0 w-5 h-5 rounded-full ${bgColor} flex items-center justify-center ${textColor} mr-2 mt-0.5`}
			>
				<Icon icon={icon} className="text-xs" />
			</div>
			<div>{text}</div>
		</div>
	);
};

// Hero Section
const HeroSection = () => {
	return (
		<section className="relative flex min-h-[60vh] flex-col justify-center px-6 pt-32 pb-24">
			<div className="mx-auto w-full max-w-7xl">
				<div className="max-w-3xl">
					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="mb-6 text-4xl font-normal leading-[1.1] tracking-tight text-white md:text-6xl"
					>
						Rivet Actors vs <br />
						Cloudflare Durable Objects
					</motion.h1>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-base leading-relaxed text-zinc-500 md:text-lg"
					>
						Cloudflare Durable Objects provide stateful serverless computing with vendor lock-in.
						Rivet Actors gives you the same capabilities as an open-source library that works with your existing infrastructure and technology stack.
					</motion.p>
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="mt-8 flex flex-col gap-3 sm:flex-row"
					>
						<a
							href="/docs/actors/quickstart/backend"
							className="selection-dark inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200"
						>
							Get Started with Rivet Actors
							<Icon icon={faArrowRight} />
						</a>
					</motion.div>
				</div>
			</div>
		</section>
	);
};

// Combined Overview and Choice Section
const CombinedOverviewSection = () => {
	const rivetChoices = [
		{
			icon: faCheck,
			title: "Developer-friendly experience",
			description:
				"When you want an intuitive platform with high-quality documentation, mature local development experience, and in-depth observability in to your workloads",
		},
		{
			icon: faCheck,
			title: "Works with your existing infrastructure",
			description:
				"When you want to use actors with your existing deployment process on Kubernetes, AWS, VPS, or any infrastructure",
		},
		{
			icon: faCheck,
			title: "Technology flexibility",
			description:
				"When you want to use your existing frameworks and libraries without platform-specific constraints",
		},
		{
			icon: faCheck,
			title: "Provides monitoring and observability",
			description:
				"When you need built-in monitoring for actors that integrates with your existing observability stack",
		},
		{
			icon: faCheck,
			title: "Rich ecosystem of integrations",
			description:
				"When you want a comprehensive ecosystem with ready-to-use integrations for popular frameworks and tools",
		},
	];

	const cloudflareChoices = [
		{
			icon: faCheck,
			title: "Already using Cloudflare ecosystem",
			description:
				"When you're already committed to Cloudflare Workers and want stateful capabilities",
		},
		{
			icon: faCheck,
			title: "JavaScript/TypeScript only",
			description:
				"When your team exclusively works with Cloudflare's limited JavaScript/TypeScript runtime and doesn't need access to the broader npm ecosystem",
		},
		{
			icon: faCheck,
			title: "Don't mind platform constraints",
			description:
				"When you're comfortable with Cloudflare's deployment process, monitoring limitations, and vendor lock-in",
		},
		{
			icon: faCheck,
			title: "Prefer low-level primitives",
			description:
				"When you want raw primitives and don't need a rich ecosystem of framework integrations",
		},
	];

	return (
		<section className="border-t border-white/10 py-24">
			<div className="mx-auto max-w-7xl px-6">
				<div className="mb-12">
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl"
					>
						Overview
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="max-w-xl text-base leading-relaxed text-zinc-500"
					>
						Compare the two approaches and decide which is right for your project.
					</motion.p>
				</div>

				<div className="grid grid-cols-1 gap-8 md:grid-cols-2">
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="flex flex-col border-t border-white/10 pt-6"
					>
						<div className="mb-3 text-zinc-500">
							<Icon icon={faRivet} className="h-4 w-4" />
						</div>
						<h3 className="mb-2 text-base font-normal text-white">Rivet Actors</h3>
						<p className="mb-6 text-sm leading-relaxed text-zinc-500">
							Rivet Actors is an open-source library that brings the actor model
							to your existing infrastructure. Build stateful, distributed
							applications with your preferred technology stack, deployed on your
							own infrastructure.
						</p>

						<h4 className="text-sm font-medium uppercase tracking-wider text-zinc-500 mb-4">
							When to choose Rivet Actors
						</h4>
						<div className="space-y-4 mb-6">
							{rivetChoices.map((choice, index) => (
								<div key={index} className="flex items-start gap-2">
									<Icon icon={choice.icon} className="h-3 w-3 text-zinc-500 mt-1" />
									<div>
										<span className="text-sm text-white">{choice.title}</span>
										<span className="text-sm text-zinc-500"> — {choice.description}</span>
									</div>
								</div>
							))}
						</div>
						<div className="mt-auto">
							<a href="/docs/actors/quickstart/backend"
								className="selection-dark inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200"
							>
								Get started with Rivet Actors
								<Icon icon={faArrowRight} />
							</a>
						</div>
					</motion.div>

					<motion.div
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="flex flex-col border-t border-white/10 pt-6"
					>
						<div className="mb-3 text-zinc-500">
							<Icon icon={faCloudflare} className="h-4 w-4" />
						</div>
						<h3 className="mb-2 text-base font-normal text-white">Cloudflare Durable Objects</h3>
						<p className="mb-6 text-sm leading-relaxed text-zinc-500">
							Cloudflare Durable Objects provide stateful serverless computing
							that runs on Cloudflare's global edge network. Built on Cloudflare's
							proprietary platform, Durable Objects offer strong consistency and
							state persistence for JavaScript/TypeScript applications.
						</p>

						<h4 className="text-sm font-medium uppercase tracking-wider text-zinc-500 mb-4">
							When to choose Cloudflare Durable Objects
						</h4>
						<div className="space-y-4">
							{cloudflareChoices.map((choice, index) => (
								<div key={index} className="flex items-start gap-2">
									<Icon icon={choice.icon} className="h-3 w-3 text-zinc-500 mt-1" />
									<div>
										<span className="text-sm text-white">{choice.title}</span>
										<span className="text-sm text-zinc-500"> — {choice.description}</span>
									</div>
								</div>
							))}
						</div>
					</motion.div>
				</div>
			</div>
		</section>
	);
};

// Consolidated Feature Comparison Table
interface FeatureGroup {
	groupTitle: string;
	features: {
		feature: string;
		rivet: {
			text: any;
			status: "yes" | "no" | "partial" | "coming-soon";
		};
		cloudflare: {
			text: string;
			status: "yes" | "no" | "partial" | "coming-soon";
		};
		importance: string;
	}[];
}

const ComparisonTable = () => {
	const featureGroups: FeatureGroup[] = [
		{
			groupTitle: "Open Source",
			features: [
				{
					feature: "Open-source",
					rivet: {
						status: "yes",
						text: (
							<>
								Yes, Rivet is open-source with the Apache 2.0
								license.{" "}
								<a href="https://github.com/rivet-dev/engine">
									View on GitHub
								</a>
								.
							</>
						),
					},
					cloudflare: {
						status: "no",
						text: "No, Cloudflare is a closed-source, proprietary platform",
					},
					importance:
						"Building your core technology on open-source software is vital to ensure portability and flexibility as your needs change",
				},
			],
		},
		{
			groupTitle: "Infrastructure",
			features: [
				{
					feature: "Works with existing infrastructure",
					rivet: {
						status: "yes",
						text: "Deploy actors on Kubernetes, AWS, VPS, or any infrastructure",
					},
					cloudflare: {
						status: "no",
						text: "Locked to Cloudflare's infrastructure",
					},
					importance:
						"Using your existing infrastructure avoids vendor lock-in and integrates with your current setup",
				},
				{
					feature: "Data sovereignty and VPC isolation",
					rivet: {
						status: "yes",
						text: "Full control over data residency and network isolation within your VPC",
					},
					cloudflare: {
						status: "no",
						text: "Data processed on Cloudflare's global network with limited control",
					},
					importance:
						"Data sovereignty ensures compliance with data governance requirements and maintains complete network isolation",
				},
				{
					feature: "Works with existing deploy processes",
					rivet: {
						status: "yes",
						text: "Import the library and deploy with your existing CI/CD",
					},
					cloudflare: {
						status: "no",
						text: "Requires Cloudflare-specific deployment process",
					},
					importance:
						"Keeping your existing deployment process reduces complexity and learning curve",
				},
				{
					feature: "Technology flexibility",
					rivet: {
						status: "yes",
						text: "Works with your existing technology stack and frameworks",
					},
					cloudflare: {
						status: "partial",
						text: "Limited to Cloudflare's limited JavaScript/TypeScript runtime, not compatible with many npm packages",
					},
					importance:
						"Technology flexibility lets you use your existing skills and codebase",
				},
				{
					feature: "Integrates with existing monitoring",
					rivet: {
						status: "yes",
						text: "Works with your existing observability stack",
					},
					cloudflare: {
						status: "partial",
						text: "Limited monitoring options, mostly Cloudflare-specific",
					},
					importance:
						"Integration with existing monitoring reduces operational overhead",
				},
			],
		},
		{
			groupTitle: "Runtime",
			features: [
				{
					feature: "Actor support",
					rivet: {
						status: "yes",
						text: "First-class actor model with Rivet Actors library",
					},
					cloudflare: {
						status: "yes",
						text: "Durable Objects for stateful workloads",
					},
					importance:
						"Actor model enables scalable stateful applications with state realtime, persistence, and realtime",
				},
				{
					feature: "KV Persistence",
					rivet: {
						status: "yes",
						text: "Built-in KV storage for actors",
					},
					cloudflare: {
						status: "yes",
						text: "KV supported for Durable Objects",
					},
					importance:
						"Key-value storage enables persistent state without external dependencies",
				},
				{
					feature: "SQLite Persistence",
					rivet: {
						status: "coming-soon",
						text: "SQLite support in preview",
					},
					cloudflare: {
						status: "yes",
						text: "SQLite supported for Durable Objects",
					},
					importance:
						"SQLite provides relational database capabilities for complex data models",
				},
				{
					feature: "Memory limits",
					rivet: {
						status: "yes",
						text: "Configurable memory limits based on needs",
					},
					cloudflare: {
						status: "partial",
						text: "128MB limit for Durable Objects",
					},
					importance:
						"Higher memory limits allow more complex stateful applications",
				},
				{
					feature: "Automatic connection handling",
					rivet: {
						status: "yes",
						text: "Optionally provides abstraction over HTTP, WebSockets, and SSE with intelligent failure and reconnection handling",
					},
					cloudflare: {
						status: "no",
						text: "Requires low-level implementation of connection management",
					},
					importance:
						"Automatic connection handling reduces development time and improves reliability",
				},
				{
					feature: "Event broadcasting",
					rivet: {
						status: "yes",
						text: "Built-in event broadcasting to specific connections or all actors",
					},
					cloudflare: {
						status: "partial",
						text: "Requires complex setup or third-party solutions like PartyKit",
					},
					importance:
						"Native event system enables real-time features with minimal setup",
				},
				{
					feature: "Built-in scheduling",
					rivet: {
						status: "yes",
						text: "Powerful built-in scheduling system",
					},
					cloudflare: {
						status: "partial",
						text: "Requires boilerplate on top of Alarms API",
					},
					importance:
						"Native scheduling reduces complexity and improves reliability for time-based operations",
				},
				{
					feature: "Testing support",
					rivet: {
						status: "yes",
						text: "Full Vitest support with mocking and fake timers",
					},
					cloudflare: {
						status: "partial",
						text: "Limited Vitest support due to custom runtime constraints",
					},
					importance:
						"Comprehensive testing capabilities ensure code quality and reliability",
				},
				{
					feature: "Customizable actor lifecycle",
					rivet: {
						status: "yes",
						text: "Flexible draining mechanism with configurable lifecycle management",
					},
					cloudflare: {
						status: "partial",
						text: "60s grace period",
					},
					importance:
						"Customizable lifecycle management allows for graceful state transfers and prevents data loss",
				},
				{
					feature: "Control over actor upgrades",
					rivet: {
						status: "yes",
						text: "Full control based on your existing rollout mechanisms",
					},
					cloudflare: {
						status: "no",
						text: "Only allows controlling gradual deployment percentages, not specific Durable Object versions",
					},
					importance:
						"Controlled upgrades ensure smooth transitions without service disruption tailored to your application's architecture",
				},
				{
					feature: "Actor creation with input data",
					rivet: {
						status: "yes",
						text: "Pass initialization data when creating actors",
					},
					cloudflare: {
						status: "no",
						text: "Cannot pass input data during Durable Object creation",
					},
					importance:
						"Ability to initialize actors with data simplifies setup and reduces boilerplate",
				},
				{
					feature: "Actor shutdown control",
					rivet: {
						status: "yes",
						text: "Clean shutdown API for actors",
					},
					cloudflare: {
						status: "partial",
						text: "Requires deleteAll with custom logic and error-prone boilerplate",
					},
					importance:
						"Proper shutdown control ensures graceful cleanup and prevents resource leaks",
				},
				{
					feature: "Monitoring",
					rivet: {
						status: "yes",
						text: "Built-in monitoring for development and production",
					},
					cloudflare: {
						status: "no",
						text: "Custom monitoring required",
					},
					importance:
						"Integrated monitoring simplifies operations and debugging",
				},
				{
					feature: "Logging",
					rivet: {
						status: "yes",
						text: "Suports your existing logging infrastructure",
					},
					cloudflare: {
						status: "no",
						text: "Provides no logging for Durable Objects",
					},
					importance:
						"Built-in logging reduces setup time and operational complexity",
				},
				{
					feature: "Metadata access",
					rivet: {
						status: "yes",
						text: "Built-in metadata API",
					},
					cloudflare: {
						status: "no",
						text: "Custom implementation required",
					},
					importance:
						"Direct access to metadata such as tags, region, and more simplifies management and deployment",
				},
				// {
				// 	feature: "REST API",
				// 	rivet: {
				// 		status: "yes",
				// 		text: "Full REST API for actor management",
				// 	},
				// 	cloudflare: {
				// 		status: "no",
				// 		text: "No RESTful API for Durable Objects",
				// 	},
				// 	importance:
				// 		"REST API enables programmatic management and integration with external tools",
				// },
				// {
				// 	feature: "Actor discovery",
				// 	rivet: {
				// 		status: "yes",
				// 		text: "Flexible tagging system for organizing, querying, and monitoring actors",
				// 	},
				// 	cloudflare: {
				// 		status: "partial",
				// 		text: "String-based lookup",
				// 	},
				// 	importance:
				// 		"Tagging enables more sophisticated service discovery patterns",
				// },
			],
		},
		// {
		// 	groupTitle: "Platform",
		// 	features: [
		// 		{
		// 			feature: "Instant rollback to versions",
		// 			rivet: {
		// 				status: "yes",
		// 				text: "One-click rollback to previous versions",
		// 			},
		// 			cloudflare: {
		// 				status: "yes",
		// 				text: "Version rollback supported",
		// 			},
		// 			importance:
		// 				"Quick rollback capabilities minimize downtime and recover from problematic deployments",
		// 		},
		// 		{
		// 			feature: "Built-in monitoring & logging",
		// 			rivet: {
		// 				status: "yes",
		// 				text: "Comprehensive monitoring and logging for all services",
		// 			},
		// 			cloudflare: {
		// 				status: "partial",
		// 				text: "Limited for Workers, not supported for Durable Objects",
		// 			},
		// 			importance:
		// 				"Integrated monitoring and logging simplifies troubleshooting and performance optimization",
		// 		},
		// 		{
		// 			feature: "User-uploaded builds",
		// 			rivet: {
		// 				status: "yes",
		// 				text: "Full support for user-uploaded builds and multi-tenancy",
		// 			},
		// 			cloudflare: {
		// 				status: "yes",
		// 				text: "Cloudflare for Platforms",
		// 			},
		// 			importance:
		// 				"Enables building platforms where your users can upload their own code to run on your infrastructure",
		// 		},
		// 		{
		// 			feature: "Tagging for builds, actors, and containers",
		// 			rivet: {
		// 				status: "yes",
		// 				text: "Comprehensive tagging system for all resources",
		// 			},
		// 			cloudflare: {
		// 				status: "no",
		// 				text: "No built-in tagging system",
		// 			},
		// 			importance:
		// 				"Tagging is important for organization, cost allocation, and managing user-uploaded builds",
		// 		},
		// 	],
		// },
		{
			groupTitle: "Developer Tooling",
			features: [
				{
					feature: "State inspector",
					rivet: {
						status: "yes",
						text: "Built-in tools to inspect and modify actor state",
					},
					cloudflare: {
						status: "no",
						text: "No built-in state inspection tools",
					},
					importance:
						"Ability to view & edit actor state in real time simplifies debugging and management",
				},
				{
					feature: "RPC debugger",
					rivet: {
						status: "yes",
						text: "Interactive RPC testing tools for actors",
					},
					cloudflare: {
						status: "no",
						text: "No built-in RPC debugging",
					},
					importance:
						"Ability to test remote procedure calls to actors accelerates development and troubleshooting",
				},
				{
					feature: "Connection inspector",
					rivet: {
						status: "yes",
						text: "Real-time monitoring of actor connections",
					},
					cloudflare: {
						status: "no",
						text: "No connection visualization tools",
					},
					importance:
						"Visibility into active connections helps diagnose client-side issues and monitor usage patterns",
				},
				{
					feature: "Actor listing and management",
					rivet: {
						status: "yes",
						text: "Browse and manage active actors with full interaction capabilities",
					},
					cloudflare: {
						status: "partial",
						text: "Can list Durable Objects but cannot interact with them",
					},
					importance:
						"Being able to list and interact with live actors enables debugging and operational management",
				},
			],
		},
		{
			groupTitle: "Development Experience",
			features: [
				{
					feature: "Documentation",
					rivet: {
						status: "yes",
						text: "Comprehensive, developer-focused documentation",
					},
					cloudflare: {
						status: "partial",
						text: "Fragmented and difficult to understand documentation",
					},
					importance:
						"Clear documentation accelerates learning and implementation",
				},
				// {
				// 	feature: "Local development with multiple apps",
				// 	rivet: {
				// 		status: "yes",
				// 		text: "Unified local development experience for managing multiple apps",
				// 	},
				// 	cloudflare: {
				// 		status: "no",
				// 		text: "Requires tmux or similar for running multiple Wrangler instances in parallel",
				// 	},
				// 	importance:
				// 		"Local development experience for multiple apps (i.e. microservices) reduces developer friction with configuration & improves developer collaboration.",
				// },
				{
					feature: "Compatible with Docker Compose",
					rivet: {
						status: "yes",
						text: "Seamless integration with Docker Compose for local development",
					},
					cloudflare: {
						status: "no",
						text: "No Docker Compose compatibility",
					},
					importance:
						"Integration with Docker Compose enables use with your existing development workflows and tools",
				},
				// {
				// 	feature: "Observability for debugging",
				// 	rivet: {
				// 		status: "yes",
				// 		text: "Built-in obervability with tools available both localy and in Rivet Cloud",
				// 	},
				// 	cloudflare: {
				// 		status: "no",
				// 		text: "Requires additional setup",
				// 	},
				// 	importance:
				// 		"Immediate visibility into application behavior speeds debugging",
				// },
			],
		},
	];

	return (
		<div className="overflow-x-auto border-t border-white/10">
			<table className="w-full border-collapse [&_a]:underline [&_a]:text-white [&_a]:hover:text-zinc-300">
				<thead>
					<tr className="border-b border-white/10">
						<th className="py-3 px-4 text-left text-xs font-medium uppercase tracking-wider text-zinc-500">
							Feature
						</th>
						<th className="py-3 px-4 text-left text-xs font-medium">
							<div className="flex items-center gap-2">
								<Icon icon={faRivet} className="text-zinc-500" />
								<span className="text-white">Rivet Actors</span>
							</div>
						</th>
						<th className="py-3 px-4 text-left text-xs font-medium">
							<div className="flex items-center gap-2">
								<Icon icon={faCloudflare} className="text-zinc-500" />
								<span className="text-zinc-500">Cloudflare Durable Objects</span>
							</div>
						</th>
						<th className="py-3 px-4 text-left text-xs font-medium uppercase tracking-wider text-zinc-500">
							Why it matters
						</th>
					</tr>
				</thead>
				<tbody>
					{featureGroups.map((group, groupIndex) => (
						<React.Fragment key={groupIndex}>
							{/* Group header row */}
							<tr className="bg-zinc-900/50 border-b border-white/10">
								<td
									colSpan={4}
									className="py-2 px-4 text-sm font-medium text-white"
								>
									{group.groupTitle}
								</td>
							</tr>
							{/* Feature rows for this group */}
							{group.features.map((feature, featureIndex) => (
								<tr
									key={`${groupIndex}-${featureIndex}`}
									className="border-b border-white/5 hover:bg-white/5 transition-colors"
								>
									<td className="py-3 px-4 text-sm text-white">
										{feature.feature}
									</td>
									<td className="py-3 px-4 text-sm text-zinc-400">
										<FeatureStatus
											status={feature.rivet.status}
											text={feature.rivet.text}
										/>
									</td>
									<td className="py-3 px-4 text-sm text-zinc-500">
										<FeatureStatus
											status={
												feature.cloudflare.status
											}
											text={feature.cloudflare.text}
										/>
									</td>
									<td className="py-3 px-4 text-sm text-zinc-500">
										{feature.importance}
									</td>
								</tr>
							))}
						</React.Fragment>
					))}
				</tbody>
			</table>
		</div>
	);
};

// Feature Comparison Section Wrapper
const ComparisonSection = () => {
	return (
		<section className="border-t border-white/10 py-24">
			<div className="mx-auto max-w-7xl px-6">
				<div className="mb-12">
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl"
					>
						Feature Comparison
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="max-w-xl text-base leading-relaxed text-zinc-500"
					>
						A detailed breakdown of capabilities across both platforms.
					</motion.p>
				</div>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.2 }}
				>
					<ComparisonTable />
				</motion.div>
			</div>
		</section>
	);
};

// Migration Section
const MigrationSection = () => {
	return (
		<section className="border-t border-white/10 py-24 text-center">
			<div className="mx-auto max-w-3xl px-6">
				<motion.h2
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="mb-3 text-2xl font-normal tracking-tight text-white md:text-4xl"
				>
					Migrating from Cloudflare Durable Objects?
				</motion.h2>
				<motion.p
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.1 }}
					className="mb-6 text-base leading-relaxed text-zinc-500"
				>
					Our team can help make the transition smooth and seamless. We
					provide migration assistance, technical guidance, and dedicated
					support.
				</motion.p>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.2 }}
				>
					<a href="/talk-to-an-engineer"
						className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white"
					>
						Talk to an engineer
					</a>
				</motion.div>
			</div>
		</section>
	);
};

// Conclusion Section
const ConclusionSection = () => {
	return (
		<section className="border-t border-white/10 py-24 text-center">
			<div className="mx-auto max-w-3xl px-6">
				<motion.h2
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="mb-3 text-2xl font-normal tracking-tight text-white md:text-4xl"
				>
					Conclusion
				</motion.h2>
				<motion.p
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.1 }}
					className="text-base leading-relaxed text-zinc-500"
				>
					While Cloudflare Durable Objects provides stateful serverless
					computing with vendor lock-in, Rivet Actors offers the same actor
					model capabilities as an open-source library that works with your
					existing infrastructure. Choose Rivet Actors when you want the power
					of actors without changing your deployment process, technology stack,
					or being locked into a specific platform.
				</motion.p>
			</div>
		</section>
	);
};

// CTA Section
const CTASection = () => {
	return (
		<section className="border-t border-white/10 px-6 py-48 text-center">
			<div className="mx-auto max-w-3xl">
				<motion.h2
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="mb-6 text-2xl font-normal tracking-tight text-white md:text-4xl"
				>
					Infrastructure for software that thinks.
				</motion.h2>
				<motion.p
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.1 }}
					className="mb-8 text-base leading-relaxed text-zinc-500"
				>
					The next generation of software needs a new kind of backend. This is it.
				</motion.p>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.2 }}
					className="flex flex-col items-center justify-center gap-3 sm:flex-row"
				>
					<a
						href="/docs"
						className="selection-dark inline-flex items-center justify-center whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200"
					>
						Start Building
					</a>
					<a
						href="/talk-to-an-engineer"
						className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white"
					>
						Talk to an Engineer
					</a>
				</motion.div>
			</div>
		</section>
	);
};

// Main Page Component
export default function RivetVsCloudflareWorkersPage() {
	return (
		<div className="min-h-screen bg-black font-sans text-zinc-300 selection:bg-[#FF4500]/30 selection:text-orange-200">
			<main>
				<HeroSection />
				<CombinedOverviewSection />
				<ComparisonSection />
				<ConclusionSection />
				<MigrationSection />
				<CTASection />
			</main>
		</div>
	);
}
