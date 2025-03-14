import apiPages from "@/generated/apiPages.json" assert { type: "json" };
import type { Sitemap } from "@/lib/sitemap";
import {
	faDownload,
	faExclamationTriangle,
	faGlobe,
	faSquareInfo,
	faTs,
} from "@rivet-gg/icons";

// Goals:
// - Siebar links should advertise the product, collapse any advanced pages away
// - The sidebar should be 1 screen height when collapsed

// Profiles:
// - What does Rivet do?
//	- Does it work for my use cases -> Use Cases
//	- Curious about the technology -> Build with Rivet
// - Just want to jump in
// - People who want to run Open Source

export const sitemap = [
	{
		title: "Documentation",
		href: "/docs",
		sidebar: [
			{
				title: "Overview",
				href: "/docs",
				icon: faSquareInfo,
			},
			{
				title: "Install CLI",
				href: "/docs/install",
				icon: faDownload,
			},
			{
				title: "Getting Started", // See https://supabase.com/docs/guides/auth/quickstarts/nextjs
				pages: [
					{
						title: "TypeScript",
						href: "/docs/quickstart/typescript",
						icon: faTs,
					},
				],
			},
			{
				title: "Guides",
				pages: [
					{
						title: "Realtime Chat App",
						href: "/guides/chat",
					},
					//{
					//	title: "Collaborative App with Y.js",
					//	href: "/guides/yjs-nextjs",
					//},
					//{
					//	title: "AI Agent with AI SDK",
					//	href: "/guides/ai-agent-ai-sdk",
					//},
					//{
					//	title: "Multiplayer Game with Three.js",
					//	href: "/guides/multiplayer-threejs",
					//},
					//{
					//	title: "Durable Execution",
					//	href: "/guides/durable-execution",
					//},
					//{
					//	title: "Stateful Stream Processing with Kafka",
					//	href: "/guides/stream-processing-kafka",
					//},
					//{
					//	title: "Local-First Sync with TinyBase",
					//	href: "/guides/local-first-sync-tinybase",
					//},
					//{
					//	title: "Sandboxed Code Execution for Python",
					//	href: "/guides/sandboxed-code-python",
					//},
					//{
					//	title: "Sandboxed Code Execution for TypeScript",
					//	href: "/guides/sandboxed-code-typescript",
					//},
					//{
					//	title: "Authenticating using Supabase Auth",
					//	href: "/guides/authenticate-supabase-auth",
					//},
				],
			},
			{
				title: "Reference",
				pages: [
					{
						title: "Configuration",
						href: "/docs/config",
						//icon: faSquareSliders,
					},
					{
						title: "Local Development",
						href: "/docs/local-development",
						//icon: faCode
					},
					{
						title: "Troubleshooting",
						href: "/docs/troubleshooting",
						//icon: faClipboardListCheck,
					},
					{
						title: "FAQ",
						href: "/docs/faq",
						//icon: faBlockQuestion,
					},
					{
						title: "Self-Hosting",
						collapsible: true,
						pages: [
							{
								title: "Overview",
								href: "/docs/self-hosting",
							},
							{
								title: "Single Container",
								href: "/docs/self-hosting/single-container",
							},
							{
								title: "Docker Compose",
								href: "/docs/self-hosting/docker-compose",
							},
							{
								title: "Manual Deployment",
								href: "/docs/self-hosting/manual-deployment",
							},
							{
								title: "Server Config",
								href: "/docs/self-hosting/server-config",
							},
							{
								title: "Client Config",
								href: "/docs/self-hosting/client-config",
							},
						],
					},
					{
						title: "More",
						collapsible: true,
						pages: [
							{
								title: "Edge Regions",
								href: "/docs/edge",
								icon: faGlobe,
							},
							{
								title: "Limitations",
								href: "/docs/limitations",
								icon: faExclamationTriangle,
							},
							{
								title: "Advanced",
								collapsible: true,
								pages: [
									{
										title: "Runtime",
										href: "/docs/runtime",
									},
									{
										title: "Networking",
										href: "/docs/networking",
									},
									{
										title: "Rescheduling",
										href: "/docs/rescheduling",
									},
								],
							},
						],
					},
				],
			},
			//{
			//	title: "Integrations",
			//	pages: [
			//		{
			//			title: "AI Agents",
			//			collapsible: true,
			//			pages: [
			//				{ title: "LangGraph", href: "/docs/integrations/tinybase" },
			//			]
			//		},
			//		{
			//			title: "Local-First Sync",
			//			collapsible: true,
			//			pages: [
			//				{ title: "TinyBase", href: "/docs/integrations/tinybase" },
			//			]
			//		},
			//	],
			//},
		],
	},
	{
		title: "Integrations",
		href: "/integrations",
		sidebar: [
			{
				title: "Introduction",
				href: "/integrations",
				icon: faSquareInfo,
			},
			//{
			//	title: "AI Agents",
			//	pages: [
			//		{ title: "LangGraph", href: "/integrations/tinybase" },
			//	],
			//},
			//{
			//	title: "Local-First Sync",
			//	pages: [
			//		{ title: "TinyBase", href: "/integrations/tinybase" },
			//	],
			//},
			{
				title: "Monitoring",
				pages: [
					{
						title: "Better Stack",
						href: "/integrations/better-stack",
					},
				],
			},
		],
	},
	{
		title: "API",
		href: "/docs/api",
		sidebar: apiPages.pages,
	},
] satisfies Sitemap;
