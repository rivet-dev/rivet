"use client";

import React from "react";
import {
	Icon,
	faRocket,
	faFeather,
	faServer,
	faDatabase,
	faGlobe,
	faNetworkWired,
	faGears,
	faCloud,
	faCode,
	faArrowRight,
	faTerminal,
	faShieldAlt,
	faBroadcastTower,
	faCodeBranch,
	faChartLine,
	faTools,
	faUserCog,
	faLaptopCode,
	faWindowRestore,
	faClock,
	faBoxOpen,
	faClipboardList,
	faSearch,
	faExchangeAlt,
	faLifeRing,
	faHdd,
	faEnvelope,
	faLock,
	faShieldCheck,
	faDatabase as faDatabaseAlt,
	faCog,
	faDollarSign,
	faFileExport,
	faSyncAlt,
	faCheck,
	faRivet,
	faCloudflare,
	faXmark,
	faMinus,
	faHourglass,
	faFunction,
	faActorsBorderless,
} from "@rivet-gg/icons";
import Link from "next/link";
import Image from "next/image";
import imgHub from "@/images/screenshots/rivet-hub.png";
import { CtaSection } from "../CtaSection";

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
					Rivet vs Cloudflare Workers
				</h1>
				<p className="text-xl text-white/70 max-w-3xl mx-auto">
					Cloudflare revolutionized serverless computing with Workers.
					<br />
					Rivet takes it to the next level with an open-source &
					developer-friendly platform.
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
								style={{ objectFit: "cover", objectPosition: "top" }}
								className="w-full h-auto"
							/>
						</div>
					</div>
					<div className="relative h-[1px] bg-white/20 z-20 w-[120%] -left-[10%]" style={{ marginTop: "-1px", position: "relative" }} />
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
			title: "Flexible deployment options",
			description:
				"When you need the flexibility to deploy to either Rivet Cloud or self-host on-premises",
		},
		{
			icon: faCheck,
			title: "Works with mainstream technologies",
			description:
				"When you need things to work out of the box without fuss using Node.js, Python, Bun, and more instead of proprietary runtimes",
		},
		{
			icon: faCheck,
			title: "Open-source",
			description:
				"When you want freedom from vendor lock-in and the benefits of an open-source community under the Apache 2.0 license",
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
			title: "Global CDN is a priority",
			description:
				"When content delivery and edge caching are your primary requirements",
		},
		{
			icon: faCheck,
			title: "Tight Cloudflare ecosystem integration",
			description:
				"When you need seamless integration with Cloudflare's existing services and security features",
		},
		{
			icon: faCheck,
			title: "Prefer Cloudflare's proprietary runtime",
			description:
				"When you prefer Cloudflare's specific runtime environment over Node.js, Python, and your tools of choice",
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
						<h3 className="text-2xl font-medium text-white">
							Rivet
						</h3>
					</div>
					<p className="text-white/70 mb-8">
						Rivet simplifies building complex applications with an
						open-source and developer-first platform. With Rivet,
						developers can easily build and deploy Functions,
						Actors, and Containers — all with the language and tools
						they already know.
					</p>
					
					<h4 className="text-xl font-medium text-white mb-6">When to choose Rivet</h4>
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
							href="https://hub.rivet.gg"
							className="inline-flex items-center px-4 py-2 bg-white text-black rounded-md hover:bg-white/90 transition-colors"
						>
							Get started with Rivet{" "}
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
							Cloudflare Workers
						</h3>
					</div>
					<p className="text-white/70 mb-8">
						Cloudflare Workers is a serverless platform that lets
						you execute code at the edge across Cloudflare's global
						network of data centers. Built on Cloudflare's
						proprietary JavaScript runtime, Workers provides
						powerful functionality for edge computing, caching, and
						stateful workloads.
					</p>
					
					<h4 className="text-xl font-medium text-white mb-6">When to choose Cloudflare Workers</h4>
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
								<Link href="https://github.com/rivet-gg/rivet">
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
			groupTitle: "Hosting Options",
			features: [
				{
					feature: "Cloud hosting",
					rivet: {
						status: "yes",
						text: (
							<>
								Rivet Cloud with{" "}
								<Link href="/docs/edge">
									global edge network
								</Link>
							</>
						),
					},
					cloudflare: {
						status: "yes",
						text: "Global edge network of data centers",
					},
					importance:
						"Managed cloud hosting simplifies operations and maintenance",
				},
				{
					feature: "Choose underlying cloud provider",
					rivet: {
						status: "yes",
						text: (
							<>
								Choice of cloud providers for Rivet Cloud (
								<Link href="/sales">contact us</Link>)
							</>
						),
					},
					cloudflare: {
						status: "no",
						text: "Limited to Cloudflare's infrastructure",
					},
					importance:
						"Choosing your cloud provider allows you to ensure you can run your backend in the same datacenter as your database & other services",
				},
				{
					feature: "Bring-your-own-cloud deployment",
					rivet: {
						status: "yes",
						text: (
							<>
								Support for hybrid cloud/on-premises deployment
								(<Link href="/sales">contact us</Link>)
							</>
						),
					},
					cloudflare: {
						status: "no",
						text: "Cloud-only solution",
					},
					importance:
						"Hybrid options enable flexible compliance with regulatory requirements",
				},
				{
					feature: "Self-hosting",
					rivet: {
						status: "yes",
						text: (
							<>
								Full support for{" "}
								<Link href="/docs/self-hosting">
									self-hosting on any infrastructure
								</Link>
							</>
						),
					},
					cloudflare: {
						status: "no",
						text: "No self-hosting option",
					},
					importance:
						"Self-hosting provides control over infrastructure and data sovereignty",
				},
				{
					feature: "Deployment portability",
					rivet: {
						status: "yes",
						text: "Move between self-hosted and cloud seamlessly",
					},
					cloudflare: {
						status: "no",
						text: "Locked to Cloudflare infrastructure",
					},
					importance:
						"Portability prevents vendor lock-in and enables flexible deployment options",
				},
			],
		},
		{
			groupTitle: "Core Runtime",
			features: [
				{
					feature: "Programming languages",
					rivet: {
						status: "yes",
						text: "JavaScript, TypeScript, Python, Rust, and any language that can run in Docker",
					},
					cloudflare: {
						status: "partial",
						text: "JavaScript, TypeScript, Python (limited support via WASM), Rust (via WASM)",
					},
					importance:
						"More language options provide flexibility for diverse team skills and use cases",
				},
				{
					feature: "Local development",
					rivet: {
						status: "yes",
						text: "Fully consistent local environment with 1:1 production parity",
					},
					cloudflare: {
						status: "partial",
						text: "Wrangler with some production differences",
					},
					importance:
						"Predictable local development reduces bugs and deployment surprises",
				},
				{
					feature: "Pricing model",
					rivet: {
						status: "yes",
						text: "Pay for CPU & memory with predictable scaling",
					},
					cloudflare: {
						status: "partial",
						text: "Pay-per-request with complex variables",
					},
					importance:
						"Transparent pricing helps with forecasting and budgeting",
				},
			],
		},
		{
			groupTitle: "Rivet Functions vs Cloudflare Workers",
			features: [
				{
					feature: "Automatic SSL Management",
					rivet: {
						status: "yes",
						text: "Built-in SSL certificate management",
					},
					cloudflare: {
						status: "yes",
						text: "SSL certificates included",
					},
					importance:
						"Automatic SSL management simplifies secure deployments",
				},
				{
					feature: "DDoS Mitigation",
					rivet: {
						status: "yes",
						text: "Advanced DDoS protection built-in",
					},
					cloudflare: {
						status: "yes",
						text: "Includes DDoS protection",
					},
					importance:
						"Protection against distributed denial of service attacks ensures application availability",
				},
				{
					feature: "Runs at edge",
					rivet: {
						status: "yes",
						text: "Global edge deployment with low latency",
					},
					cloudflare: {
						status: "yes",
						text: "Extensive global edge network",
					},
					importance:
						"Edge computing reduces latency and improves user experience globally",
				},
				{
					feature: "Cold starts",
					rivet: {
						status: "yes",
						text: "Optimized cold starts with predictable performance",
					},
					cloudflare: {
						status: "yes",
						text: "Fast cold start times",
					},
					importance:
						"Fast cold starts ensure consistent user experience without delays",
				},
				{
					feature: "Memory limits",
					rivet: {
						status: "yes",
						text: "Configurable based on workload needs",
					},
					cloudflare: {
						status: "partial",
						text: "128MB limit for Workers",
					},
					importance:
						"Higher memory limits enable more complex processing within a single function",
				},
				{
					feature: "CPU time limits",
					rivet: {
						status: "yes",
						text: "Unrestricted",
					},
					cloudflare: {
						status: "partial",
						text: "10ms (free), 5m (paid)",
					},
					importance:
						"Generous CPU time allows for more complex operations without timing out",
				},
			],
		},
		{
			groupTitle: "Rivet Actors vs Cloudflare Durable Objects",
			features: [
				{
					feature: "Actor support",
					rivet: {
						status: "yes",
						text: "First-class actor model with Rivet Actors",
					},
					cloudflare: {
						status: "yes",
						text: "Durable Objects for stateful workloads",
					},
					importance:
						"Actor model enables scalable stateful applications with state persistence, hibernation, and realtime",
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
					feature: "REST API",
					rivet: {
						status: "yes",
						text: "Full REST API for actor management",
					},
					cloudflare: {
						status: "no",
						text: "No RESTful API for Durable Objects",
					},
					importance:
						"REST API enables programmatic management and integration with external tools",
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
				{
					feature: "Graceful shutdown",
					rivet: {
						status: "yes",
						text: "3-hour draining window",
					},
					cloudflare: {
						status: "partial",
						text: "60s grace period",
					},
					importance:
						"Draining period allows for graceful state transfers and prevents data loss",
				},
				{
					feature: "Connection protocols",
					rivet: {
						status: "yes",
						text: "HTTP, WebSockets, TCP, and UDP support",
					},
					cloudflare: {
						status: "partial",
						text: "HTTP and WebSockets only",
					},
					importance:
						"More protocol options support diverse application requirements",
				},
				{
					feature: "Actor networking",
					rivet: {
						status: "yes",
						text: "Actors have dedicated hostnames that can be accessed via Functions or directly (without a middle man)",
					},
					cloudflare: {
						status: "partial",
						text: "Durable Objects have limited networking functionality, can only be access via Workers",
					},
					importance:
						"Customizable networking enables more flexibility in why types of workloads your actors can serve & compatibility with existing tooling",
				},
				{
					feature: "Actor discovery",
					rivet: {
						status: "yes",
						text: "Flexible tagging system for organizing, querying, and monitoring actors",
					},
					cloudflare: {
						status: "partial",
						text: "String-based lookup",
					},
					importance:
						"Tagging enables more sophisticated service discovery patterns",
				},
				{
					feature: "Control over actor upgrades",
					rivet: {
						status: "yes",
						text: (
							<>
								Full control with{" "}
								<Link href="/docs/api/actors/upgrade">
									dedicated upgrade APIs
								</Link>
							</>
						),
					},
					cloudflare: {
						status: "no",
						text: "Only allows controlling gradual deployment percentages, not specific Durable Object versions",
					},
					importance:
						"Controlled upgrades ensure smooth transitions without service disruption tailored to your application's architecture",
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
						text: "Comprehensive logging included",
					},
					cloudflare: {
						status: "no",
						text: "Complex setup depending on configuration",
					},
					importance:
						"Built-in logging reduces setup time and operational complexity",
				},
			],
		},
		{
			groupTitle: "Rivet Containers vs Cloudflare Containers",
			features: [
				{
					feature: "Container support",
					rivet: {
						status: "yes",
						text: "Rivet supports containers",
					},
					cloudflare: {
						status: "coming-soon",
						text: "Cloudflare Containers are in development, details TBD",
					},
					importance:
						"Supports launching containers on-demand for use cases such as batch jobs, game servers, media encoding, and more",
				},
			],
		},
		{
			groupTitle: "Platform",
			features: [
				{
					feature: "Instant rollback to versions",
					rivet: {
						status: "yes",
						text: "One-click rollback to previous versions",
					},
					cloudflare: {
						status: "yes",
						text: "Version rollback supported",
					},
					importance:
						"Quick rollback capabilities minimize downtime and recover from problematic deployments",
				},
				{
					feature: "Built-in monitoring & logging",
					rivet: {
						status: "yes",
						text: "Comprehensive monitoring and logging for all services",
					},
					cloudflare: {
						status: "partial",
						text: "Limited for Workers, not supported for Durable Objects",
					},
					importance:
						"Integrated monitoring and logging simplifies troubleshooting and performance optimization",
				},
				{
					feature: "User-uploaded builds",
					rivet: {
						status: "yes",
						text: "Full support for user-uploaded builds and multi-tenancy",
					},
					cloudflare: {
						status: "yes",
						text: "Cloudflare for Platforms",
					},
					importance:
						"Enables building platforms where your users can upload their own code to run on your infrastructure",
				},
				{
					feature: "Tagging for builds, actors, and containers",
					rivet: {
						status: "yes",
						text: "Comprehensive tagging system for all resources",
					},
					cloudflare: {
						status: "no",
						text: "No built-in tagging system",
					},
					importance:
						"Tagging is important for organization, cost allocation, and managing user-uploaded builds",
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
				{
					feature: "Local development with multiple apps",
					rivet: {
						status: "yes",
						text: "Unified local development experience for managing multiple apps",
					},
					cloudflare: {
						status: "no",
						text: "Requires tmux or similar for running multiple Wrangler instances in parallel",
					},
					importance:
						"Local development experience for multiple apps (i.e. microservices) reduces developer friction with configuration & improves developer collaboration.",
				},
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
				{
					feature: "Observability for debugging",
					rivet: {
						status: "yes",
						text: "Built-in obervability with tools available both localy and in Rivet Cloud",
					},
					cloudflare: {
						status: "no",
						text: "Requires additional setup",
					},
					importance:
						"Immediate visibility into application behavior speeds debugging",
				},
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
									<span className="text-white">Rivet</span>
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
										Cloudflare Workers
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
										className={`border-b border-zinc-800 ${featureIndex % 2 === 0 ? "bg-zinc-900/30" : "bg-[#0A0A0A]"}`}
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
			<h2 className="text-2xl font-medium text-white mb-6">Migrating from Cloudflare Workers or have questions?</h2>
			<div className="prose prose-invert max-w-none">
				<p>
					Looking to migrate your existing Cloudflare Workers applications to Rivet or have more questions about how Rivet can meet your needs? Our team can help make the transition smooth and seamless. We provide migration assistance, technical guidance, and dedicated support to ensure your experience with Rivet is successful.
				</p>
				<div className="mt-6">
					<Link
						href="/sales"
						className="inline-flex items-center px-5 py-2.5 bg-white text-black rounded-md hover:bg-white/90 transition-colors no-underline"
					>
						Contact Us
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
					While Cloudflare Workers excels at global CDN capabilities
					and edge computing, Rivet offers an open-source and more
					developer-friendly experience with flexible deployment
					options, comprehensive local development tools, and powerful
					stateful models through its actor system. Choose Rivet for
					complex applications that benefit from open-source
					flexibility, or Cloudflare when tight CDN integration is
					your priority.
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
				<CtaSection />
			</div>
		</div>
	);
}