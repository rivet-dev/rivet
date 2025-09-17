"use client";

import {
	Icon,
	faArrowsTurnRight,
	faClock,
	faCode,
	faCogs,
	faCourtSport,
	faDatabase,
	faFileCode,
	faFilePdf,
	faGamepad,
	faImage,
	faLinear,
	faOven,
	faPhone,
	faRobot,
	faServer,
	faSync,
	faWaveSine,
} from "@rivet-gg/icons";
import Image from "next/image";
import { useEffect, useRef, useState } from "react";

import imgBatchJobs from "./images/diagrams/batch-jobs.svg";
// Import all diagram images
import imgCodingAgent from "./images/diagrams/coding-agent.svg";
import imgCollaborativeDocuments from "./images/diagrams/collaborative-documents.svg";
import imgEtlPipeline from "./images/diagrams/etl-pipeline.svg";
import imgFfmpegConversion from "./images/diagrams/ffmpeg-conversion.svg";
import imgLinearAgent from "./images/diagrams/linear-agent.svg";
import imgLiveEvents from "./images/diagrams/live-events.svg";
import imgLocalFirstSync from "./images/diagrams/local-first-sync.svg";
import imgManagedCi from "./images/diagrams/managed-ci.svg";
import imgMcpServer from "./images/diagrams/mcp-server.svg";
import imgMultiplayerGameServers from "./images/diagrams/multiplayer-game-servers.svg";
import imgPdfTextExtraction from "./images/diagrams/pdf-text-extraction.svg";
import imgPerTenantDatabase from "./images/diagrams/per-tenant-database.svg";
import imgSandboxedCode from "./images/diagrams/sandboxed-code.svg";
import imgScheduledTasks from "./images/diagrams/scheduled-tasks.svg";
import imgStreamProcessing from "./images/diagrams/stream-processing.svg";
import imgVoiceAgent from "./images/diagrams/voice-agent.svg";

interface TabItem {
	id: string;
	icon: any;
	title: string;
	description: string;
	image: any;
	docs?: {
		actors?: boolean;
		containers?: boolean;
		functions?: boolean;
		guide?: string;
		sourceCode?: string;
	};
}

interface TabGroup {
	id: string;
	title: string;
	items: TabItem[];
}

// Define all tab groups and their items at the top level
const TAB_GROUPS: TabGroup[] = [
	{
		id: "scaling",
		title: "Scaling",
		items: [
			{
				id: "foundationdb",
				icon: faDatabase,
				title: "FoundationDB Backend",
				description:
					"Enterprise-grade distributed database that provides ACID transactions and infinite scalability for actor state management.",
				image: imgPerTenantDatabase, // Reuse existing image
				docs: {
					actors: true,
					containers: false,
					functions: false,
				},
			},
			{
				id: "multi-region",
				icon: faServer,
				title: "Multi-Region Deployment",
				description:
					"Deploy actors across multiple geographic regions with automatic failover and data replication.",
				image: imgLocalFirstSync, // Reuse existing image
				docs: {
					actors: true,
					containers: false,
					functions: false,
				},
			},
			{
				id: "fault-tolerance",
				icon: faCogs,
				title: "Fault Tolerance",
				description:
					"Built-in error handling and recovery with automatic actor restart and state preservation.",
				image: imgManagedCi, // Reuse existing image
				docs: {
					actors: true,
					containers: false,
					functions: false,
				},
			},
		],
	},
	{
		id: "ai",
		title: "AI",
		items: [
			{
				id: "coding-agent",
				icon: faRobot,
				title: "Coding Agent",
				description:
					"Deploy intelligent coding assistants that help developers write, review, and optimize code.",
				image: imgCodingAgent,
				docs: {
					actors: true,
					containers: true,
					functions: true,
				},
			},
			{
				id: "linear-agent",
				icon: faLinear,
				title: "Linear Agent",
				description:
					"Deploy intelligent Linear Agent that integrates with customers' Linear workspaces.",
				image: imgLinearAgent,
				docs: {
					actors: true,
					containers: false,
					functions: true,
					guide: "/blog/2025-05-28-building-linear-agents-in-node-js-and-rivet-full-walkthrough-and-starter-kit",
					sourceCode:
						"https://github.com/rivet-gg/rivet/tree/e13e6e95c56ea63bc73312fa7d01a647412ac507/examples/linear-agent-starter",
				},
			},
			{
				id: "ai-voice",
				icon: faPhone,
				title: "Realtime Voice Agent",
				description:
					"Create voice-enabled AI assistants with natural language processing capabilities.",
				image: imgVoiceAgent,
				docs: {
					actors: true,
				},
			},
			{
				id: "sandboxed-code",
				icon: faCode,
				title: "Sandboxed Code Execution",
				description:
					"Execute untrusted code securely in isolated environments with resource limitations and security controls.",
				image: imgSandboxedCode,
				docs: {
					containers: true,
					functions: true,
				},
			},
			{
				id: "mcp",
				icon: faServer,
				title: "MCP Server",
				description:
					"Run mission control platform servers for centralized monitoring and management.",
				image: imgMcpServer,
				docs: {
					actors: true,
					containers: false,
					functions: false,
				},
			},
		],
	},
	{
		id: "realtime",
		title: "Realtime",
		items: [
			{
				id: "local-first",
				icon: faSync,
				title: "Local-First Sync",
				description:
					"Build applications that work offline and seamlessly sync when connection is restored.",
				image: imgLocalFirstSync,
				docs: {
					actors: true,
					containers: false,
					functions: false,
				},
			},
			{
				id: "collaborative",
				icon: faFileCode,
				title: "Collaborative Documents",
				description:
					"Create realtime collaborative editing experiences like Google Docs or Figma.",
				image: imgCollaborativeDocuments,
				docs: {
					actors: true,
					containers: false,
					functions: false,
				},
			},
			{
				id: "multiplayer",
				icon: faGamepad,
				title: "Multiplayer Game Servers",
				description:
					"Host low-latency game servers that scale automatically with player demand.",
				image: imgMultiplayerGameServers,
				docs: {
					actors: false,
					containers: true,
					functions: false,
				},
			},
			{
				id: "live-events",
				icon: faCourtSport,
				title: "Live Events",
				description:
					"Create interactive live events with realtime participation for thousands of concurrent users.",
				image: imgLiveEvents,
				docs: {
					actors: true,
					containers: false,
					functions: false,
				},
			},
		],
	},
	{
		id: "infra",
		title: "Infrastructure",
		items: [
			{
				id: "per-tenant",
				icon: faDatabase,
				title: "Per-Tenant Databases",
				description:
					"Isolate each customer's data in dedicated database instances for improved security and performance.",
				image: imgPerTenantDatabase,
				docs: {
					actors: true,
					containers: false,
					functions: false,
				},
			},
			{
				id: "ci",
				icon: faCogs,
				title: "Managed CI",
				description:
					"Automate your build, test, and deployment workflow with managed continuous integration.",
				image: imgManagedCi,
				docs: {
					actors: false,
					containers: true,
					functions: true,
				},
			},
			{
				id: "analytics",
				icon: faOven,
				title: "Batch Jobs",
				description:
					"Process large datasets in parallel with efficient resource allocation and automatic scaling.",
				image: imgBatchJobs,
				docs: {
					actors: true,
					containers: false,
					functions: true,
				},
			},
			{
				id: "scheduled",
				icon: faClock,
				title: "Scheduled Background Tasks",
				description:
					"Schedule and run background tasks at specified intervals with reliable execution.",
				image: imgScheduledTasks,
				docs: {
					actors: true,
					containers: false,
					functions: false,
				},
			},
			//{
			//	id: "microservices",
			//	icon: faNetworkWired,
			//	title: "Microservices",
			//	description:
			//		"TODO",
			//	imagePath: "/path/to/ci.svg", // Replace with actual SVG path
			//},
		],
	},
	{
		id: "data",
		title: "Data",
		items: [
			{
				id: "etl",
				icon: faArrowsTurnRight,
				title: "ETL Pipeline",
				description:
					"Build data pipelines that extract, transform, and load data between various sources and destinations.",
				image: imgEtlPipeline,
				docs: {
					actors: true,
					containers: false,
					functions: false,
				},
			},
			{
				id: "stream",
				icon: faWaveSine,
				title: "Realtime Stream Processing",
				description:
					"Process large volumes of streaming data in realtime with high throughput.",
				image: imgStreamProcessing,
				docs: {
					actors: true,
					containers: false,
					functions: true,
				},
			},
		],
	},
	{
		id: "media",
		title: "Media",
		items: [
			{
				id: "ffmpeg",
				icon: faImage,
				title: "FFmpeg Image Conversion",
				description:
					"Process and convert images and videos at scale with industry-standard tools.",
				image: imgFfmpegConversion,
				docs: {
					actors: false,
					containers: true,
					functions: true,
				},
			},
			{
				id: "pdf",
				icon: faFilePdf,
				title: "PDF Text Extraction",
				description:
					"Extract and process text from PDF documents efficiently at scale.",
				image: imgPdfTextExtraction,
				docs: {
					actors: false,
					containers: true,
					functions: true,
				},
			},
			//{
			//	id: "analytics",
			//	icon: faChartLine,
			//	title: "Data Analytics Processing",
			//	description:
			//		"Analyze large datasets and generate insights with parallel processing.",
			//	imagePath: "/path/to/analytics.svg", // Replace with actual SVG path
			//},
		],
	},
];

// Component for documentation links
const DocLink = ({
	title,
	href,
	external,
}: { title: string; href: string; external?: boolean }) => (
	<a
		href={href}
		className="text-white/70 hover:text-white text-sm font-medium flex items-center rounded-md bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 px-3 py-1.5 transition-colors"
		target={external ? "_blank" : undefined}
	>
		{title}
		<span className="ml-1.5 text-white/50">→</span>
	</a>
);

export const PowerfulPrimitivesSection = () => {
	const [activeTab, setActiveTab] = useState<string>(
		TAB_GROUPS[0].items[0].id,
	);
	const [showLeftArrow, setShowLeftArrow] = useState(false);
	const [showRightArrow, setShowRightArrow] = useState(true);

	const tabsContainerRef = useRef<HTMLDivElement>(null);

	// Create a flat array of all tabs for finding the active tab
	const allTabs = TAB_GROUPS.flatMap((group) => group.items);

	// Find the active tab data
	const activeTabData =
		allTabs.find((tab) => tab.id === activeTab) || allTabs[0];

	// Check scroll position and update arrow visibility
	const checkScroll = () => {
		const container = tabsContainerRef.current;
		if (!container) return;

		setShowLeftArrow(container.scrollLeft > 0);
		setShowRightArrow(
			container.scrollLeft <
				container.scrollWidth - container.clientWidth - 5, // 5px buffer
		);
	};

	// Set up scroll event listener
	useEffect(() => {
		const container = tabsContainerRef.current;
		if (container) {
			container.addEventListener("scroll", checkScroll);
			// Initial check
			checkScroll();

			// Check after content/window might have changed
			const resizeObserver = new ResizeObserver(checkScroll);
			resizeObserver.observe(container);

			return () => {
				container.removeEventListener("scroll", checkScroll);
				resizeObserver.disconnect();
			};
		}
	}, []);

	// Scroll handlers
	const scrollLeft = () => {
		const container = tabsContainerRef.current;
		if (container) {
			// Calculate one third of the container width for more substantial scroll
			const scrollAmount = Math.max(350, container.clientWidth / 2);
			container.scrollBy({ left: -scrollAmount, behavior: "smooth" });
		}
	};

	const scrollRight = () => {
		const container = tabsContainerRef.current;
		if (container) {
			// Calculate one third of the container width for more substantial scroll
			const scrollAmount = Math.max(350, container.clientWidth / 2);
			container.scrollBy({ left: scrollAmount, behavior: "smooth" });
		}
	};

	return (
		<div className="mx-auto max-w-7xl py-32 lg:py-40">
			{/* Centered title and subtitle outside the box */}
			<div className="text-center mb-12">
				<h2 className="text-4xl font-medium tracking-tight text-white">
					Powerful primitives to build anything
				</h2>
				<p className="mt-4 text-lg text-white/70">
					Choose the right building blocks for your specific use case
				</p>
			</div>

			{/* Box Container - no border */}
			<div className="bg-[#0A0A0A] rounded-xl overflow-hidden">
				<div className="flex flex-col lg:flex-row">
					{/* Left Column - Tab Buttons - 50% width on desktop */}
					<div className="w-full lg:w-1/2 p-5">
						{/* Tab Buttons - 2 column grid layout */}
						<div className="grid grid-cols-2 gap-5">
							{/* Map through each tab group */}
							{TAB_GROUPS.map((group) => (
								<div key={group.id}>
									<h4 className="text-white mb-2 text-xs tracking-wider px-3 font-medium">
										{group.title}
									</h4>
									<div className="space-y-1.5">
										{group.items.map((tab) => (
											<button
												key={tab.id}
												className={`w-full flex items-center justify-start gap-2 px-3.5 py-2.5 rounded-md transition-all text-xs ${
													activeTab === tab.id
														? "bg-zinc-900 text-white border border-zinc-500 shadow-inner shadow-zinc-800"
														: "text-white/50 hover:text-white hover:bg-zinc-800/60 border border-transparent hover:border-zinc-700/50"
												}`}
												onClick={() =>
													setActiveTab(tab.id)
												}
											>
												<Icon
													icon={tab.icon}
													className="text-sm"
												/>
												<span>{tab.title}</span>
											</button>
										))}
									</div>
								</div>
							))}
						</div>
					</div>

					{/* Right Column - Details Content - 50% width on desktop */}
					<div className="w-full lg:w-1/2 p-5">
						{/* Card Layout */}
						<div className="bg-zinc-900 rounded-xl overflow-hidden border border-zinc-800">
							{/* Title and Diagram Section - Same Background */}
							<div className="bg-[#020202]">
								{/* Title and Description */}
								<div className="p-5">
									<h3 className="text-xl font-medium text-white mb-3">
										{activeTabData.title}
									</h3>
									<p className="text-white/80">
										{activeTabData.description}
									</p>
								</div>

								{/* Image */}
								<div className="p-10 flex items-center justify-center min-h-[250px]">
									<Image
										src={activeTabData.image}
										alt={activeTabData.title}
										className="max-w-full max-h-[300px]"
										width={
											activeTabData.image.width
												? activeTabData.image.width / 2
												: undefined
										}
										height={
											activeTabData.image.height
												? activeTabData.image.height / 2
												: undefined
										}
									/>
								</div>
							</div>

							{/* Links Section */}
							{activeTabData.docs && (
								<div className="p-4">
									<div className="text-sm font-medium text-white/90 mb-2">
										Documentation
									</div>
									<div className="flex gap-3 flex-wrap">
										{activeTabData.docs.actors && (
											<DocLink
												title="Actors"
												href="/docs/actors"
											/>
										)}
										{activeTabData.docs.containers && (
											<DocLink
												title="Containers"
												href="/docs/cloud/containers"
											/>
										)}
										{activeTabData.docs.functions && (
											<DocLink
												title="Functions"
												href="/docs/cloud/functions"
											/>
										)}
										{activeTabData.docs.guide && (
											<DocLink
												title="Guide"
												href={activeTabData.docs.guide}
											/>
										)}
										{activeTabData.docs.sourceCode && (
											<DocLink
												title="Source Code"
												href={
													activeTabData.docs
														.sourceCode
												}
												external={true}
											/>
										)}
									</div>
								</div>
							)}
						</div>
					</div>
				</div>
			</div>
		</div>
	);
};
