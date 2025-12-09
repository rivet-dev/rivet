"use client";

import imgHub from "@/images/screenshots/rivet-hub.png";
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
import Image from "next/image";
import Link from "next/link";
import React from "react";
import { CTASection } from "../(index)/sections/CTASection";

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
		<div className="mx-auto max-w-7xl pt-20 pb-40 px-6 lg:px-8">
			<div className="text-center">
				<h1 className="text-5xl font-bold tracking-tight text-white mb-6">
					Rivet Actors vs Cloudflare Durable Objects
				</h1>
				<p className="text-xl text-white/70 max-w-3xl mx-auto">
					Cloudflare Durable Objects provide stateful serverless computing with
					vendor lock-in.
					<br />
					Rivet Actors gives you the same capabilities as an
					open-source library that works with your existing
					infrastructure and technology stack.
				</p>
			</div>

			<div className="h-28" />

			<div className="mx-auto relative">
				<div className="max-w-[1200px] w-full mx-auto relative">
					<div className="relative rounded-t-lg border border-b-0 border-white/10 overflow-hidden">
						<div className="w-full max-w-full aspect-[16/9] md:aspect-[2/1] relative">
							<Image
								src={imgHub}
								alt="Rivet Hub dashboard"
								fill
								sizes="(max-width: 640px) 100vw, (max-width: 768px) 100vw, 1200px"
								style={{
									objectFit: "cover",
									objectPosition: "top",
								}}
								className="w-full h-auto"
							/>
						</div>
					</div>
					<div
						className="relative h-[1px] bg-white/20 z-20 w-[120%] -left-[10%]"
						style={{ marginTop: "-1px", position: "relative" }}
					/>
				</div>
			</div>
		</div>
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
		//{
		//	icon: faCheck,
		//	title: "Full local development experience",
		//	description:
		//		"When you value a seamless local development workflow with 1:1 production parity",
		//},
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
		<div className="w-full px-2 py-12 bg-white/2 border border-white/15 rounded-lg">
			<h2 className="text-3xl font-medium text-center tracking-tight text-white mb-12">
				Overview
			</h2>
			<div className="grid grid-cols-1 lg:grid-cols-2 lg:divide-x lg:divide-white/10 gap-y-12 lg:gap-y-0 h-full">
				<div className="px-8 h-full flex flex-col">
					<div className="flex items-center mb-4">
						<div className="w-12 h-12 rounded-xl bg-white/5 flex items-center justify-center text-white mr-4">
							<Icon icon={faRivet} className="text-2xl" />
						</div>
						<h3 className="text-2xl font-medium text-white">Rivet Actors</h3>
					</div>
					<p className="text-white/70 mb-8">
						Rivet Actors is an open-source library that brings the actor model
						to your existing infrastructure. Build stateful, distributed
						applications with your preferred technology stack, deployed on your
						own infrastructure.
					</p>

					<h4 className="text-xl font-medium text-white mb-6">
						When to choose Rivet Actors
					</h4>
					<div className="space-y-6 flex-grow">
						{rivetChoices.map((choice, index) => (
							<div key={index} className="flex gap-4">
								<div className="flex-shrink-0 h-10 w-10 flex items-center justify-center text-green-500">
									<Icon
										icon={choice.icon}
										className="text-xl"
									/>
								</div>
								<div>
									<h3 className="text-lg font-medium text-white mb-1">
										{choice.title}
									</h3>
									<p className="text-white/70">
										{choice.description}
									</p>
								</div>
							</div>
						))}
					</div>
					<div className="mt-10 text-center">
						<Link
							href="/docs/actors/quickstart/backend"
							className="inline-flex items-center px-4 py-2 bg-white text-black rounded-md hover:bg-white/90 transition-colors"
						>
							Get started with Rivet Actors{" "}
							<Icon icon={faArrowRight} className="ml-2" />
						</Link>
					</div>
				</div>
				<div className="px-8 h-full flex flex-col pt-6 lg:pt-0 border-t border-white/10 lg:border-t-0">
					<div className="flex items-center mb-4">
						<div className="w-12 h-12 rounded-xl bg-white/5 flex items-center justify-center text-white/70 mr-4">
							<Icon icon={faCloudflare} className="text-2xl" />
						</div>
						<h3 className="text-2xl font-medium text-white">
							Cloudflare Durable Objects
						</h3>
					</div>
					<p className="text-white/70 mb-8">
						Cloudflare Durable Objects provide stateful serverless computing
						that runs on Cloudflare's global edge network. Built on Cloudflare's
						proprietary platform, Durable Objects offer strong consistency and
						state persistence for JavaScript/TypeScript applications.
					</p>

					<h4 className="text-xl font-medium text-white mb-6">
						When to choose Cloudflare Durable Objects
					</h4>
					<div className="space-y-6 flex-grow">
						{cloudflareChoices.map((choice, index) => (
							<div key={index} className="flex gap-4">
								<div className="flex-shrink-0 h-10 w-10 flex items-center justify-center text-green-500">
									<Icon
										icon={choice.icon}
										className="text-xl"
									/>
								</div>
								<div>
									<h3 className="text-lg font-medium text-white mb-1">
										{choice.title}
									</h3>
									<p className="text-white/70">
										{choice.description}
									</p>
								</div>
							</div>
						))}
					</div>
				</div>
			</div>
		</div>
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
								<Link href="https://github.com/rivet-dev/engine">
									View on GitHub
								</Link>
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
		<div className="mt-16 mb-20">
			<div className="overflow-x-auto">
				<table className="w-full border-collapse [&_a]:underline">
					<thead>
						<tr className="bg-zinc-900 border-b border-zinc-800">
							<th className="py-4 px-6 text-left text-sm font-medium text-white/70">
								Feature
							</th>
							<th className="py-4 px-6 text-left text-sm font-medium">
								<div className="flex items-center">
									<div className="w-6 h-6 rounded bg-white/5 flex items-center justify-center text-white mr-2">
										<Icon
											icon={faRivet}
											className="text-xs"
										/>
									</div>
									<span className="text-white">
										Rivet Actors
									</span>
								</div>
							</th>
							<th className="py-4 px-6 text-left text-sm font-medium">
								<div className="flex items-center">
									<div className="w-6 h-6 rounded bg-white/5 flex items-center justify-center text-white/70 mr-2">
										<Icon
											icon={faCloudflare}
											className="text-xs"
										/>
									</div>
									<span className="text-white/70">
										Cloudflare Durable Objects
									</span>
								</div>
							</th>
							<th className="py-4 px-6 text-left text-sm font-medium text-white/70">
								Why it matters
							</th>
						</tr>
					</thead>
					<tbody>
						{featureGroups.map((group, groupIndex) => (
							<React.Fragment key={groupIndex}>
								{/* Group header row */}
								<tr className="bg-zinc-800">
									<td
										colSpan={4}
										className="py-3 px-6 text-md font-semibold text-white"
									>
										{group.groupTitle}
									</td>
								</tr>
								{/* Feature rows for this group */}
								{group.features.map((feature, featureIndex) => (
									<tr
										key={`${groupIndex}-${featureIndex}`}
										className={`border-b border-zinc-800 ${featureIndex % 2 === 0 ? "bg-zinc-900/30" : "bg-background"}`}
									>
										<td className="py-4 px-6 text-sm font-medium text-white">
											{feature.feature}
										</td>
										<td className="py-4 px-6 text-sm text-white">
											<FeatureStatus
												status={feature.rivet.status}
												text={feature.rivet.text}
											/>
										</td>
										<td className="py-4 px-6 text-sm text-white/70">
											<FeatureStatus
												status={
													feature.cloudflare.status
												}
												text={feature.cloudflare.text}
											/>
										</td>
										<td className="py-4 px-6 text-sm text-white/60">
											{feature.importance}
										</td>
									</tr>
								))}
							</React.Fragment>
						))}
					</tbody>
				</table>
			</div>
		</div>
	);
};

// Migration Section
const MigrationSection = () => {
	return (
		<div className="mx-auto max-w-4xl py-12 px-6 lg:px-8 bg-zinc-900/50 rounded-lg border border-white/10">
			<h2 className="text-2xl font-medium text-white mb-6">
				Migrating from Cloudflare Durable Objects or have questions?
			</h2>
			<div className="prose prose-invert max-w-none">
				<p>
					Looking to migrate your existing Cloudflare Durable Objects to Rivet
					Actors or have more questions about how Rivet Actors can meet your
					needs? Our team can help make the transition smooth and seamless. We
					provide migration assistance, technical guidance, and dedicated
					support to ensure your experience with Rivet Actors is successful.
				</p>
				<div className="mt-6">
					<Link
						href="/talk-to-an-engineer"
						className="inline-flex items-center px-5 py-2.5 bg-white text-black rounded-md hover:bg-white/90 transition-colors no-underline"
					>
						Talk to an engineer
					</Link>
				</div>
			</div>
		</div>
	);
};

// Conclusion
const Conclusion = () => {
	return (
		<div className="mx-auto max-w-4xl py-28 px-6 lg:px-8">
			<h2 className="text-3xl font-medium text-white mb-6">Conclusion</h2>
			<div className="prose prose-invert max-w-none">
				<p>
					While Cloudflare Durable Objects provides stateful serverless
					computing with vendor lock-in, Rivet Actors offers the same actor
					model capabilities as an open-source library that works with your
					existing infrastructure. Choose Rivet Actors when you want the power
					of actors without changing your deployment process, technology stack,
					or being locked into a specific platform. Choose Cloudflare Durable
					Objects when you're already committed to the Cloudflare ecosystem and
					don't mind the platform constraints.
				</p>
			</div>
		</div>
	);
};

// Main Page Component
export default function RivetVsCloudflareWorkersPage() {
	return (
		<div className="min-h-screen w-full max-w-[1500px] mx-auto px-4 md:px-8 pt-36 pb-24">
			<div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
				<HeroSection />
				<CombinedOverviewSection />

				{/* Feature Comparison Section */}
				<div className="mt-36">
					<h2 className="text-4xl font-medium text-center text-white mb-16">
						Feature Comparison
					</h2>
					<ComparisonTable />
				</div>

				<Conclusion />
				<MigrationSection />
				
				<div className="h-[1px] bg-white/20 mt-20" />
				
				<div className="py-52 sm:py-60">
					<CTASection />
				</div>
			</div>
		</div>
	);
}
