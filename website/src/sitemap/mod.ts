import apiData from "@/generated/apiPages.json" assert { type: "json" };

import {
	faArrowRightArrowLeft,
	faArrowsTurnRight,
	faArrowsTurnToDots,
	faBlockQuestion,
	faBolt,
	faClipboardListCheck,
	faClock,
	faCloud,
	faCloudArrowUp,
	faCloudflare,
	faCode,
	faCodePullRequest,
	faCoin,
	faDatabase,
	faDiagramNext,
	faDocker,
	faDownload,
	faFastForward,
	faSqlite,
	faPostgresql,
	faFileImport,
	faFingerprint,
	faFloppyDisk,
	faForward,
	faBoxesStacked,
	faGear,
	faGlobe,
	faInfoSquare,
	faKey,
	faLayerGroup,
	faLeaf,
	faListUl,
	faMaximize,
	faMemory,
	faMerge,
	faNetworkWired,
	faNextjs,
	faNodeJs,
	faPaintbrush,
	faPalette,
	faPuzzlePiece,
	faRailway,
	faRender,
	faReact,
	faRecycle,
	faRotate,
	faRust,
	faServer,
	faShareNodes,
	faSitemap,
	faSliders,
	faSlidersHSquare,
	faSquareInfo,
	faSquareRootVariable,
	faSquareSliders,
	faSquareTerminal,
	faTag,
	faTowerBroadcast,
	faSwift,
	faUpload,
	faVialCircleCheck,
	faSquareList,
	faGrid,
	faGrid2,
	faMailbox,
	faUsb,
	faUsbDrive,
	faSupabase,
} from "@rivet-gg/icons";
import { deployOptions, type DeployOption } from "@rivetkit/shared-data";
import { integrationSidebarSections } from "@/data/integrations";
import nextjs from "@/images/vendors/next-js.svg";
import type { SidebarItem, Sitemap } from "@/lib/sitemap";

const deploySidebarPages: SidebarItem[] = deployOptions.map(
	({ displayName: title, href, icon, badge }) => ({
		title,
		href,
		icon,
		badge,
	}),
);

export const sitemap = [
	{
		title: "Actors",
		href: "/docs",
		sidebar: [
			{
				title: "General",
				pages: [
					{
						title: "Introduction",
						href: "/docs/actors",
						icon: faSquareInfo,
					},
					{
						title: "Quickstart",
						icon: faFastForward,
						collapsible: true,
						pages: [
							{
								title: "Node.js & Bun",
								href: "/docs/actors/quickstart/backend",
								icon: faNodeJs,
							},
							{
								title: "React",
								href: "/docs/actors/quickstart/react",
								icon: faReact,
							},
							{
								title: "Next.js",
								href: "/docs/actors/quickstart/next-js",
								icon: faNextjs,
							},
							{
								title: "Rust",
								href: "/docs/actors/quickstart/rust",
								icon: faRust,
								badge: "Beta",
							},
							{
								title: "Effect.ts",
								href: "/docs/actors/quickstart/effect",
								icon: faLayerGroup,
								badge: "Beta",
							},
							{
								title: "Cloudflare Workers",
								href: "/docs/actors/quickstart/cloudflare",
								icon: faCloudflare,
							},
							{
								title: "Supabase Functions",
								href: "/docs/actors/quickstart/supabase",
								icon: faSupabase,
							},
						],
					},
				],
			},
			{
				title: "Features",
				pages: [
					{
						title: "In-Memory State",
						href: "/docs/actors/state",
						icon: faFloppyDisk,
					},
					{
						title: "Actions",
						href: "/docs/actors/actions",
						icon: faBolt,
					},
					{
						title: "Realtime",
						href: "/docs/actors/events",
						icon: faTowerBroadcast,
					},
					{
						title: "Workflows",
						href: "/docs/actors/workflows",
						icon: faDiagramNext,
					},
					{
						title: "Queues",
						href: "/docs/actors/queues",
						icon: faMailbox,
					},
					{
						title: "Schedule & Cron",
						href: "/docs/actors/schedule",
						icon: faClock,
					},
					{
						title: "SQLite",
						href: "/docs/actors/sqlite",
						icon: faSqlite,
					},
					// {
					// 	title: "Persistence",
					// 	collapsible: true,
					// 	icon: faDatabase,
					// 	pages: [
					// 		{
					// 			title: "Overview",
					// 			href: "/docs/actors/persistence",
					// 			icon: faGrid2
					// 		},
					// 		{
					// 			title: "SQLite",
					// 			badge: "Built-In",
					// 			href: "/docs/actors/sqlite",
					// 			icon: faSqlite,
					// 		},
					// 		{
					// 			title: "PostgreSQL",
					// 			href: "/docs/actors/postgres",
					// 			icon: faPostgresql
					// 		},
					// 	]
					// },
				],
			},
			{
				title: "Concepts",
				pages: [
					{
						title: "Crash Course",
						href: "/docs/actors/crash-course",
					},
					{
						title: "Design Patterns",
						// icon: faLayerGroup,
						href: "/docs/actors/design-patterns",
					},
					{
						title: "Communication & Networking",
						// icon: faArrowRightArrowLeft,
						collapsible: true,
						pages: [
							{
								title: "Authentication",
								href: "/docs/actors/authentication",
								//icon: faFingerprint,
							},
							{
								title: "Access Control",
								href: "/docs/actors/access-control",
							},
							{
								title: "Connections",
								href: "/docs/actors/connections",
								//icon: faNetworkWired,
							},
							{
								title: "Actor-Actor Communication",
								href: "/docs/actors/communicating-between-actors",
								//icon: faArrowsTurnToDots,
							},
							{
								title: "Low-Level WebSocket Handler",
								href: "/docs/actors/websocket-handler",
							},
							{
								title: "Low-Level HTTP Handler",
								href: "/docs/actors/request-handler",
							},
							{
								title: "Vanilla HTTP API",
								href: "/docs/actors/http-api",
							},
						],
					},
					{
						title: "Lifecycle & Config",
						// icon: faSlidersHSquare,
						collapsible: true,
						pages: [
							{
								title: "Lifecycle",
								href: "/docs/actors/lifecycle",
								//icon: faRotate,
							},
							{
								title: "Input Parameters",
								href: "/docs/actors/input",
								//icon: faFileImport,
							},
							{
								title: "Keys",
								href: "/docs/actors/keys",
								//icon: faKey,
							},
							{
								title: "Metadata",
								href: "/docs/actors/metadata",
								//icon: faTag,
							},
							{
								title: "Destroying",
								href: "/docs/actors/destroy",
								//icon: faTag,
							},
							{
								title: "Statuses",
								href: "/docs/actors/statuses",
							},
						],
					},
					{
						title: "More",
						// icon: faSitemap,
						collapsible: true,
						pages: [
							{
								title: "SQLite + Drizzle",
								href: "/docs/actors/sqlite-drizzle",
							},
							{
								title: "Logging",
								href: "/docs/general/logging",
								// icon: faListUl,
							},
							{
								title: "Errors",
								href: "/docs/actors/errors",
							},
							{
								title: "Testing",
								href: "/docs/actors/testing",
								// icon: faVialCircleCheck,
							},
							{
								title: "Debugging",
								href: "/docs/actors/debugging",
							},
							{
								title: "Types",
								href: "/docs/actors/types",
								//icon: faCode,
							},
							{
								title: "CORS",
								href: "/docs/general/cors",
								// icon: faShareNodes,
							},
							{
								title: "Versions & Upgrades",
								href: "/docs/actors/versions",
							},
							{
								title: "Icons & Names",
								href: "/docs/actors/appearance",
							},
							{
								title: "Custom Inspector Tabs",
								href: "/docs/actors/inspector-tabs",
							},
							{
								title: "Actor Runtime Socket",
								href: "/docs/actors/actor-runtime-socket",
								badge: "Beta",
							},
							{
								title: "Limits",
								href: "/docs/actors/limits",
							},
						],
					},
				],
			},
			{
				title: "Clients",
				pages: [
					{
						title: "Overview",
						href: "/docs/clients",
						// icon: faCode,
					},
					{
						title: "Languages & Frameworks",
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
							{
								title: "Swift",
								href: "/docs/clients/swift",
								icon: faSwift,
							},
							{
								title: "SwiftUI",
								href: "/docs/clients/swiftui",
								icon: faSwift,
							},
						],
					},
				],
			},
			{
				title: "Reference",
				pages: [
					// {
					// 	title: "Rivet Inspector",
					// 	href: "/docs/general/studio",
					// 	icon: faPalette,
					// },
					{
						title: "Troubleshooting",
						href: "/docs/actors/troubleshooting",
					},
					{
						title: "Production Checklist",
						href: "/docs/general/production-checklist",
					},
					{
						title: "CLI",
						href: "/docs/cli",
					},
					{
						title: "Configuration",
						collapsible: true,
						pages: [
							{
								title: "Runtime Modes",
								href: "/docs/general/runtime-modes",
							},
							{
								title: "WASM vs Native SDK",
								href: "/docs/general/wasm-vs-native-sdk",
							},
							{
								title: "Registry Configuration",
								href: "/docs/general/registry-configuration",
							},
							{
								title: "Actor Configuration",
								href: "/docs/general/actor-configuration",
							},
							{
								title: "Environment Variables",
								href: "/docs/general/environment-variables",
							},
							{
								title: "HTTP Server",
								href: "/docs/general/http-server",
							},
							{
								title: "Endpoints",
								href: "/docs/general/endpoints",
							},
						],
					},
					{
						title: "API Reference",
						collapsible: true,
						pages: [
							{
								title: "TypeScript API",
								href: "/typedoc",
								external: true,
								// icon: faSquareBinary,
							},
							{
								title: "OpenAPI",
								href: "https://github.com/rivet-dev/rivet/tree/main/rivetkit-openapi",
								external: true,
								// icon: faSquareBinary,
							},
							{
								title: "AsyncAPI",
								href: "https://github.com/rivet-dev/rivet/tree/main/rivetkit-asyncapi",
								external: true,
								// icon: faSquareBinary,
							},
						],
					},
					// {
					// 	title: "Architecture",
					// 	href: "/docs/general/architecture",
					// 	// icon: faSquareBinary,
					// },
					{
						title: "AI Integration",
						collapsible: true,
						pages: [
							{
								title: "Skill File",
								href: "/docs/general/skill",
							},
							{
								title: "Docs for LLMs",
								href: "/docs/general/docs-for-llms",
							},
						],
					},
					// {
					// 	title: "System Architecture",
					// 	href: "/docs/general/system-architecture",
					// 	icon: faLayerGroup,
					// },
				],
			},
		],
	},
	{
		// agentOS now lives on its own site. This tab links out to it instead of
		// rendering an in-site docs sidebar.
		title: "agentOS",
		href: "https://agentos-sdk.dev",
		external: true,
		target: "_blank",
	},
	// {
	// 	title: "Integrations",
	// 	href: "/integrations",
	// 	// IMPORTANT: Also update integrations/index.mdx
	// 	sidebar: [
	// 		{
	// 			title: "General",
	// 			pages: [
	// 				{
	// 					title: "Overview",
	// 					href: "/integrations",
	// 					icon: faSquareInfo,
	// 				},
	// 			]
	// 		},
	// 		...integrationSidebarSections,
	// 	],
	// },

	{
		title: "Deploy",
		href: "/docs/deploy",
		sidebar: [
			{
				title: "General",
				pages: [
					{
						title: "Introduction",
						href: "/docs/deploy",
						icon: faSquareInfo,
					},
				],
			},
			{
				title: "Platforms",
				pages: deploySidebarPages,
			},
			{
				title: "Reference",
				pages: [
					{
						title: "Pool Configuration",
						href: "/docs/general/pool-configuration",
					},
				],
			},
			{
				title: "Advanced",
				pages: [
					{
						title: "Container Runner",
						href: "/docs/deploy/container-runner",
					},
				],
			},
		],
	},
	{
		title: "Self-Hosting",
		href: "/docs/self-hosting",
		sidebar: [
			{
				title: "General",
				pages: [
					{
						title: "Overview",
						href: "/docs/self-hosting",
						// icon: faSquareInfo,
					},
					{
						title: "Install",
						href: "/docs/self-hosting/install",
						// icon: faDownload,
					},
					{
						title: "Configuration",
						href: "/docs/self-hosting/configuration",
						// icon: faGear,
					},
					{
						title: "Multi-Region",
						href: "/docs/self-hosting/multi-region",
						// icon: faGlobe,
					},
					{
						title: "TLS & Certificates",
						href: "/docs/self-hosting/tls",
					},
					{
						title: "Production Checklist",
						href: "/docs/self-hosting/production-checklist",
					},
				],
			},
			{
				title: "Platforms",
				pages: [
					{
						title: "Docker Container",
						href: "/docs/self-hosting/docker-container",
					},
					{
						title: "Docker Compose",
						href: "/docs/self-hosting/docker-compose",
					},
					{
						title: "Railway",
						href: "/docs/self-hosting/railway",
					},
					{
						title: "Render",
						href: "/docs/self-hosting/render",
					},
					{
						title: "Kubernetes",
						href: "/docs/self-hosting/kubernetes",
					},
				],
			},
			{
				title: "Persistence",
				pages: [
					{
						title: "File System",
						href: "/docs/self-hosting/filesystem",
					},
					{
						title: "PostgreSQL",
						href: "/docs/self-hosting/postgres",
					},
					{
						title: "FoundationDB",
						href: "/docs/self-hosting/foundationdb",
						badge: "Enterprise",
					},
				],
			},
			//{
			//	title: "Advanced",
			//	pages: [
			//	// TODO: Scaling
			//		// TODO: Architecture
			//		// TODO: Networking (exposed ports, how data gets routed to actors, etc)
			//	],
			//},
		],
	},
	{
		title: "Integrations",
		href: "/integrations",
		sidebar: [
			{
				title: "General",
				pages: [
					{
						title: "Overview",
						href: "/integrations",
						icon: faSquareInfo,
					},
				],
			},
			...integrationSidebarSections,
		],
	},

	// {
	// 	title: "Enterprise Cloud",
	// 	href: "/docs/cloud",
	// 	sidebar: [
	// 		{
	// 			title: "Overview",
	// 			href: "/docs/cloud",
	// 			icon: faSquareInfo,
	// 		},
	// 		{
	// 			title: "Install CLI",
	// 			href: "/docs/cloud/install",
	// 			icon: faDownload,
	// 		},
	// 		{
	// 			title: "Getting Started",
	// 			pages: [
	// 				{
	// 					title: "Functions",
	// 					href: "/docs/cloud/functions",
	// 					icon: faFunction,
	// 				},
	// 				{
	// 					title: "Actors",
	// 					href: "/docs/cloud/actors",
	// 					icon: faActorsBorderless,
	// 				},
	// 				{
	// 					title: "Containers",
	// 					href: "/docs/cloud/containers",
	// 					icon: faServer,
	// 				},
	// 			],
	// 		},
	// 		{
	// 			title: "Runtime",
	// 			pages: [
	// 				{
	// 					title: "Networking",
	// 					href: "/docs/cloud/networking",
	// 					icon: faNetworkWired,
	// 				},
	// 				{
	// 					title: "Environment Variables",
	// 					href: "/docs/cloud/environment-variables",
	// 					icon: faLeaf,
	// 				},
	// 				{
	// 					title: "Durability & Rescheduling",
	// 					href: "/docs/cloud/durability",
	// 					icon: faRecycle,
	// 				},
	// 			],
	// 		},
	// 		{
	// 			title: "Reference",
	// 			pages: [
	// 				{
	// 					title: "Configuration",
	// 					href: "/docs/cloud/config",
	// 					icon: faSquareSliders,
	// 				},
	// 				{
	// 					title: "CLI",
	// 					href: "/docs/cloud/cli",
	// 					icon: faSquareTerminal,
	// 				},
	// 				{
	// 					title: "CI/CD",
	// 					href: "/docs/cloud/continuous-delivery",
	// 					icon: faCodePullRequest,
	// 				},
	// 				{
	// 					title: "Tokens",
	// 					href: "/docs/cloud/tokens",
	// 					icon: faKey,
	// 				},
	// 				{
	// 					title: "Local Development",
	// 					href: "/docs/cloud/local-development",
	// 					icon: faCode,
	// 				},
	// 				{
	// 					title: "Edge Regions",
	// 					href: "/docs/cloud/edge",
	// 					icon: faGlobe,
	// 				},
	// 				{
	// 					title: "Billing",
	// 					href: "/docs/cloud/pricing",
	// 					icon: faCoin,
	// 				},
	// 				{
	// 					title: "Troubleshooting",
	// 					href: "/docs/cloud/troubleshooting",
	// 					icon: faClipboardListCheck,
	// 				},
	// 				{
	// 					title: "FAQ",
	// 					href: "/docs/cloud/faq",
	// 					icon: faBlockQuestion,
	// 				},
	// 			],
	// 		},
	// 		//{
	// 		//	title: "Use Cases",
	// 		//	pages: [
	// 		//		{
	// 		//			title: "Game Servers",
	// 		//			href: "/docs/cloud/solutions/game-servers",
	// 		//		},
	// 		//	],
	// 		//},
	// 		//{
	// 		//	title: "Self-Hosting",
	// 		//	pages: [
	// 		//		{
	// 		//			title: "Overview",
	// 		//			href: "/docs/cloud/self-hosting",
	// 		//			icon: faSquareInfo,
	// 		//		},
	// 		//		{
	// 		//			title: "Single Container",
	// 		//			href: "/docs/cloud/self-hosting/single-container",
	// 		//			icon: faDocker,
	// 		//		},
	// 		//		{
	// 		//			title: "Docker Compose",
	// 		//			href: "/docs/cloud/self-hosting/docker-compose",
	// 		//			icon: faDocker,
	// 		//		},
	// 		//		{
	// 		//			title: "Manual Deployment",
	// 		//			href: "/docs/cloud/self-hosting/manual-deployment",
	// 		//			icon: faGear,
	// 		//		},
	// 		//		{
	// 		//			title: "Client Config",
	// 		//			href: "/docs/cloud/self-hosting/client-config",
	// 		//			icon: faSliders,
	// 		//		},
	// 		//		{
	// 		//			title: "Server Config",
	// 		//			href: "/docs/cloud/self-hosting/server-config",
	// 		//			icon: faSliders,
	// 		//		},
	// 		//		{
	// 		//			title: "Networking",
	// 		//			href: "/docs/cloud/self-hosting/network-modes",
	// 		//			icon: faNetworkWired,
	// 		//		},
	// 		//	],
	// 		//},
	// 		{
	// 			title: "Advanced",
	// 			pages: [
	// 				{
	// 					title: "Limitations",
	// 					href: "/docs/cloud/limitations",
	// 				},
	// 			],
	// 		},
	// 		{
	// 			title: "API",
	// 			pages: [
	// 				{
	// 					title: "Overview",
	// 					collapsible: true,
	// 					pages: [
	// 						{
	// 							title: "Overview",
	// 							href: "/docs/cloud/api",
	// 						},
	// 						{
	// 							title: "Errors",
	// 							href: "/docs/cloud/api/errors",
	// 						},
	// 					],
	// 				},
	// 				...(apiData.groups as SidebarItem[]).map((x) => {
	// 					x.collapsible = true;
	// 					return x;
	// 				}),
	// 			],
	// 		},
	// 	],
	// },

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
