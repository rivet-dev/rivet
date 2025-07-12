import apiData from "@/generated/apiPages.json" assert { type: "json" };
import type { SidebarItem, Sitemap } from "@/lib/sitemap";
import {
	faActorsBorderless,
	faArrowRightArrowLeft,
	faBlockQuestion,
	faBolt,
	faClipboardListCheck,
	faClock,
	faCode,
	faCodePullRequest,
	faCoin,
	faDatabase,
	faDownload,
	faFingerprint,
	faFloppyDisk,
	faFunction,
	faGlobe,
	faKey,
	faLeaf,
	faListUl,
	faMaximize,
	faNetworkWired,
	faNodeJs,
	faReact,
	faRecycle,
	faRotate,
	faRust,
	faServer,
	faShareNodes,
	faSquareInfo,
	faSquareSliders,
	faSquareTerminal,
	faTag,
	faTowerBroadcast,
	faVialCircleCheck,
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
				title: "Actors",
				icon: faActorsBorderless,
				pages: [
					{
						title: "Overview",
						href: "/docs/actors",
						icon: faSquareInfo,
					},
					{
						title: "Quickstart",
						icon: faFunction,
						collapsible: true,
						pages: [
							{
								title: "Backend",
								href: "/docs/actors/quickstart-backend",
								icon: faNodeJs,
							},
							{
								title: "React",
								href: "/docs/actors/quickstart-react",
								icon: faReact,
							},
						],
					},
					{
						title: "State",
						href: "/docs/actors/state",
						icon: faFloppyDisk,
					},
					{
						title: "Actions",
						href: "/docs/actors/actions",
						icon: faBolt,
					},
					{
						title: "Events",
						href: "/docs/actors/events",
						icon: faTowerBroadcast,
					},
					{
						title: "Schedule",
						href: "/docs/actors/schedule",
						icon: faClock,
					},
					{
						title: "More",
						collapsible: true,
						pages: [
							{
								title: "Communicating with Actors",
								href: "/docs/actors/communicating-with-actors",
								icon: faArrowRightArrowLeft,
							},
							{
								title: "Communicating between Actors",
								href: "/docs/actors/communicating-between-actors",
								icon: faArrowRightArrowLeft,
							},
							{
								title: "Connections",
								href: "/docs/actors/connections",
								icon: faNetworkWired,
							},
							{
								title: "Lifecycle",
								href: "/docs/actors/lifecycle",
								icon: faRotate,
							},
							{
								title: "Metadata",
								href: "/docs/actors/metadata",
								icon: faTag,
							},
							{
								title: "Helper Types",
								href: "/docs/actors/helper-types",
								icon: faCode,
							},
							{
								title: "External SQL",
								href: "/docs/actors/external-sql",
								icon: faDatabase,
							},
							{
								title: "Scaling",
								href: "/docs/actors/scaling",
								icon: faMaximize,
							},
						],
					},
				],
			},
			{
				title: "Integrations",
				// IMPORTANT: Also update integrations/index.mdx
				pages: [
					{
						title: "Overview",
						href: "/docs/integrations",
						icon: faSquareInfo,
					},
					{
						title: "Frontend & Clients",
						icon: faCode,
						collapsible: true,
						pages: [
							{
								title: "JavaScript",
								href: "/docs/clients/javascript",
								icon: faNodeJs,
							},
							{
								title: "React",
								href: "/docs/clients/react",
								icon: faReact,
							},
							{
								title: "Rust",
								href: "/docs/clients/rust",
								icon: faRust,
							},
						],
					},
					{
						title: "Backend",
						icon: faServer,
						collapsible: true,
						pages: [
							{
								title: "Hono",
								href: "/docs/integrations/hono",
							},
							{
								title: "Express",
								href: "/docs/integrations/express",
							},
							{
								title: "Elysia",
								href: "/docs/integrations/elysia",
							},
							{
								title: "tRPC",
								href: "/docs/integrations/trpc",
							},
						],
					},
					{
						title: "Auth",
						icon: faKey,
						collapsible: true,
						pages: [
							{
								title: "Better Auth",
								href: "/docs/integrations/better-auth",
							},
						],
					},
					{
						title: "Misc",
						collapsible: true,
						pages: [
							{
								title: "Vitest",
								href: "/docs/integrations/vitest",
							},
						],
					},
				],
			},
			{
				title: "Reference",
				pages: [
					{
						title: "Authentication",
						href: "/docs/general/authentication",
						icon: faFingerprint,
					},
					{
						title: "Testing",
						href: "/docs/general/testing",
						icon: faVialCircleCheck,
					},
					{
						title: "More",
						collapsible: true,
						pages: [
							{
								title: "Edge",
								href: "/docs/general/edge",
								icon: faGlobe,
							},
							{
								title: "CORS",
								href: "/docs/general/cors",
								icon: faShareNodes,
							},
							{
								title: "Logging",
								href: "/docs/general/logging",
								icon: faListUl,
							},
						],
					},
				],
			},
		],
	},
	{
		title: "Cloud",
		href: "/docs/cloud",
		sidebar: [
			{
				title: "Overview",
				href: "/docs/cloud",
				icon: faSquareInfo,
			},
			{
				title: "Install CLI",
				href: "/docs/cloud/install",
				icon: faDownload,
			},
			{
				title: "Getting Started",
				pages: [
					{
						title: "Functions",
						href: "/docs/cloud/functions",
						icon: faFunction,
					},
					{
						title: "Actors",
						href: "/docs/cloud/actors",
						icon: faActorsBorderless,
					},
					{
						title: "Containers",
						href: "/docs/cloud/containers",
						icon: faServer,
					},
				],
			},
			{
				title: "Runtime",
				pages: [
					{
						title: "Networking",
						href: "/docs/cloud/networking",
						icon: faNetworkWired,
					},
					{
						title: "Environment Variables",
						href: "/docs/cloud/environment-variables",
						icon: faLeaf,
					},
					{
						title: "Durability & Rescheduling",
						href: "/docs/cloud/durability",
						icon: faRecycle,
					},
				],
			},
			{
				title: "Reference",
				pages: [
					{
						title: "Configuration",
						href: "/docs/cloud/config",
						icon: faSquareSliders,
					},
					{
						title: "CLI",
						href: "/docs/cloud/cli",
						icon: faSquareTerminal,
					},
					{
						title: "CI/CD",
						href: "/docs/cloud/continuous-delivery",
						icon: faCodePullRequest,
					},
					{
						title: "Tokens",
						href: "/docs/cloud/tokens",
						icon: faKey,
					},
					{
						title: "Local Development",
						href: "/docs/cloud/local-development",
						icon: faCode,
					},
					{
						title: "Edge Regions",
						href: "/docs/cloud/edge",
						icon: faGlobe,
					},
					{
						title: "Billing",
						href: "/docs/cloud/pricing",
						icon: faCoin,
					},
					{
						title: "Troubleshooting",
						href: "/docs/cloud/troubleshooting",
						icon: faClipboardListCheck,
					},
					{
						title: "FAQ",
						href: "/docs/cloud/faq",
						icon: faBlockQuestion,
					},
				],
			},
			{
				title: "Use Cases",
				pages: [
					{
						title: "Game Servers",
						href: "/docs/cloud/solutions/game-servers",
					},
				],
			},
			{
				title: "Advanced",
				pages: [
					{
						title: "Limitations",
						href: "/docs/cloud/limitations",
					},
					{
						title: "Self-Hosting",
						collapsible: true,
						pages: [
							{
								title: "Overview",
								href: "/docs/cloud/self-hosting",
							},
							{
								title: "Single Container",
								href: "/docs/cloud/self-hosting/single-container",
							},
							{
								title: "Docker Compose",
								href: "/docs/cloud/self-hosting/docker-compose",
							},
							{
								title: "Network Modes",
								href: "/docs/cloud/self-hosting/network-modes",
							},
							{
								title: "Manual Deployment",
								href: "/docs/cloud/self-hosting/manual-deployment",
							},
							{
								title: "Server Config",
								href: "/docs/cloud/self-hosting/server-config",
							},
							{
								title: "Client Config",
								href: "/docs/cloud/self-hosting/client-config",
							},
						],
					},
				],
			},
			{
				title: "API",
				pages: [
					{
						title: "Overview",
						collapsible: true,
						pages: [
							{
								title: "Overview",
								href: "/docs/cloud/api",
							},
							{
								title: "Errors",
								href: "/docs/cloud/api/errors",
							},
						],
					},
					...(apiData.groups as SidebarItem[]).map((x) => {
						x.collapsible = true;
						return x;
					}),
				],
			},
		],
	},
	// {
	// 	title: "Integrations",
	// 	href: "/integrations",
	// 	sidebar: [
	// 		{
	// 			title: "Introduction",
	// 			href: "/integrations",
	// 			icon: faSquareInfo,
	// 		},
	// 		//{
	// 		//	title: "AI Agents",
	// 		//	pages: [
	// 		//		{ title: "LangGraph", href: "/integrations/tinybase" },
	// 		//	],
	// 		//},
	// 		//{
	// 		//	title: "Local-First Sync",
	// 		//	pages: [
	// 		//		{ title: "TinyBase", href: "/integrations/tinybase" },
	// 		//	],
	// 		//},
	// 		{
	// 			title: "Monitoring",
	// 			pages: [
	// 				{
	// 					title: "Better Stack",
	// 					href: "/integrations/better-stack",
	// 				},
	// 			],
	// 		},
	// 	],
	// },
] satisfies Sitemap;
