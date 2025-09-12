import apiData from "@/generated/apiPages.json" assert { type: "json" };
import nextjs from "@/images/vendors/next-js.svg";
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
	faDocker,
	faDownload,
	faFingerprint,
	faFloppyDisk,
	faFunction,
	faGear,
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
	faSliders,
	faSquareInfo,
	faSquareSliders,
	faSquareTerminal,
	faTag,
	faTowerBroadcast,
	faVialCircleCheck,
	faForward,
	faSquareBinary,
	faLink,
	faMerge,
	faMemory,
	faArrowsTurnToDots,
	faArrowsTurnRight,
	faFileImport,
	faSlidersHSquare,
	faArrowsLeftRight,
	faSitemap,
	faScrewdriverWrench,
	faInfoSquare,
	faPaintbrush,
	faPalette,
	faLayerGroup,
	faVercel,
    faSquareRootVariable,
    faCloudflare,
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
		title: "Pricing",
		href: "/pricing",
	},
	{
		title: "Documentation",
		href: "/docs",
		sidebar: [
			{
				title: "Actors",
				//icon: faActorsBorderless,
				pages: [
					{
						title: "Overview",
						href: "/docs/actors",
						icon: faSquareInfo,
					},
					{
						title: "Quickstart",
						icon: faForward,
						collapsible: true,
						href: "/docs/actors/quickstart",
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
								icon: nextjs,
							},
							{
								title: "Cloudflare Workers",
								href: "/docs/actors/quickstart/cloudflare-workers",
								icon: faCloudflare,
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
								title: "Clients",
								href: "/docs/actors/clients",
								icon: faCode,
							},
							{
								title: "Lifecycle & Config",
								icon: faSlidersHSquare,
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
										title: "Helper Types",
										href: "/docs/actors/helper-types",
										//icon: faCode,
									},
								],
							},
							{
								title: "Communication",
								icon: faArrowRightArrowLeft,
								collapsible: true,
								pages: [
									{
										title: "Authentication",
										href: "/docs/actors/authentication",
										//icon: faFingerprint,
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
										title: "Fetch & WebSocket Handler",
										href: "/docs/actors/fetch-and-websocket-handler",
										//icon: faLink,
									},
								],
							},
							{
								title: "State Management",
								icon: faDatabase,
								collapsible: true,
								pages: [
									{
										title: "Ephemeral Variables",
										href: "/docs/actors/ephemeral-variables",
										//icon: faMemory,
									},
									{
										title: "Sharing & Joining State",
										href: "/docs/actors/sharing-and-joining-state",
										//icon: faMerge,
									},
									{
										title: "External SQL",
										href: "/docs/actors/external-sql",
										//icon: faDatabase,
									},
								],
							},
							{
								title: "Architecture",
								icon: faSitemap,
								collapsible: true,
								pages: [
									{
										title: "Scaling & Concurrency",
										href: "/docs/actors/scaling",
										//icon: faMaximize,
									},
								],
							},
							{
								title: "Testing",
								href: "/docs/actors/testing",
								icon: faVialCircleCheck,
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
								title: "Next.js",
								href: "/docs/clients/next-js",
								icon: nextjs,
							},
							{
								title: "Rust",
								href: "/docs/clients/rust",
								icon: faRust,
							},
							{
								title: "OpenAPI",
								href: "/docs/clients/openapi",
								icon: faFileImport,
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
							{
								title: "Next.js",
								href: "/docs/integrations/next-js",
							},
							{
								title: "Cloudflare Workers",
								href: "/docs/integrations/cloudflare-workers",
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
							{
								title: "Pino",
								href: "/docs/integrations/pino",
							},
						],
					},
				],
			},
			//{
			//	title: "Self-Hosting",
			//	// IMPORTANT: Also update integrations/index.mdx
			//	pages: [
			//		{
			//			title: "Overview",
			//			icon: faInfoSquare,
			//			href: "/docs/general/self-hosting",
			//		},
			//		{
			//			title: "Hosting Providers",
			//			icon: faServer,
			//			collapsible: true,
			//			pages: [
			//				{
			//					title: "Railway",
			//					href: "/docs/hosting-providers/railway",
			//				},
			//				// TODO: AWS ECS
			//				// TODO: Vercel
			//				{
			//					title: "Cloudflare Workers",
			//					href: "/docs/hosting-providers/cloudflare-workers",
			//				},
			//				{
			//					title: "Rivet Cloud (Enterprise)",
			//					href: "/docs/hosting-providers/rivet-cloud",
			//				},
			//
			//				// TODO: Hetzner
			//				// TODO: AWS
			//				// TODO: Cloudflare Workers
			//				// TODO: Railway
			//				// TODO: Coolify
			//				// TODO: Rivet
			//			],
			//		},
			//		{
			//			title: "Drivers",
			//			icon: faScrewdriverWrench,
			//			collapsible: true,
			//			pages: [
			//				{
			//					title: "Redis",
			//					href: "/docs/drivers/redis",
			//				},
			//				{
			//					title: "File System",
			//					href: "/docs/drivers/file-system",
			//				},
			//				{
			//					title: "Memory",
			//					href: "/docs/drivers/memory",
			//				},
			//				{
			//					title: "Build Your Own",
			//					href: "/docs/drivers/build-your-own",
			//				},
			//			],
			//		},
			//	],
			//},
			{
				title: "Reference",
				pages: [
					{
						title: "Rivet Studio",
						href: "/docs/general/studio",
						icon: faPalette,
					},
					{
						title: "Docs for LLMs",
						href: "/docs/general/docs-for-llms",
						icon: faSquareBinary,
					},
					{
						title: "System Architecture",
						href: "/docs/general/system-architecture",
						icon: faLayerGroup,
					},
					{
						title: "More",
						collapsible: true,
						pages: [
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
		title: "Self-Hosting",
		href: "/docs/self-hosting",
		sidebar: [
			{
				title: "General",
				pages: [
					{
						title: "Overview",
						href: "/docs/self-hosting",
						icon: faSquareInfo,
					},
					{
						title: "Install",
						href: "/docs/self-hosting/install",
						icon: faDownload,
					},
					{
						title: "Connect Backend",
						href: "/docs/self-hosting/connect-backend",
						icon: faNetworkWired,
					},
					{
						title: "Configuration",
						href: "/docs/self-hosting/configuration",
						icon: faGear,
					},
					{
						title: "Multi-Region",
						href: "/docs/self-hosting/multi-region",
						icon: faGlobe,
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
						title: "Kubernetes",
						href: "/docs/self-hosting/kubernetes",
					},
					{
						title: "AWS Fargate",
						href: "/docs/self-hosting/aws-fargate",
					},
					{
						title: "Google Cloud Run",
						href: "/docs/self-hosting/google-cloud-run",
					},
					{
						title: "Hetzner",
						href: "/docs/self-hosting/hetzner",
					},
				],
			},
			//{
			//	title: "Advanced",
			//	pages: [
			//	// TODO: Scaling
			//		// TODO: Architecture
			//		// TODO: Networking (expoed ports, how data gets routed to actors, etc)
			//	],
			//},
		],
	},
	{
		title: "Enterprise Cloud",
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
			//{
			//	title: "Use Cases",
			//	pages: [
			//		{
			//			title: "Game Servers",
			//			href: "/docs/cloud/solutions/game-servers",
			//		},
			//	],
			//},
			//{
			//	title: "Self-Hosting",
			//	pages: [
			//		{
			//			title: "Overview",
			//			href: "/docs/cloud/self-hosting",
			//			icon: faSquareInfo,
			//		},
			//		{
			//			title: "Single Container",
			//			href: "/docs/cloud/self-hosting/single-container",
			//			icon: faDocker,
			//		},
			//		{
			//			title: "Docker Compose",
			//			href: "/docs/cloud/self-hosting/docker-compose",
			//			icon: faDocker,
			//		},
			//		{
			//			title: "Manual Deployment",
			//			href: "/docs/cloud/self-hosting/manual-deployment",
			//			icon: faGear,
			//		},
			//		{
			//			title: "Client Config",
			//			href: "/docs/cloud/self-hosting/client-config",
			//			icon: faSliders,
			//		},
			//		{
			//			title: "Server Config",
			//			href: "/docs/cloud/self-hosting/server-config",
			//			icon: faSliders,
			//		},
			//		{
			//			title: "Networking",
			//			href: "/docs/cloud/self-hosting/network-modes",
			//			icon: faNetworkWired,
			//		},
			//	],
			//},
			{
				title: "Advanced",
				pages: [
					{
						title: "Limitations",
						href: "/docs/cloud/limitations",
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
