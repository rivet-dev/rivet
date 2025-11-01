import apiData from "@/generated/apiPages.json" assert { type: "json" };

import {
	faActorsBorderless,
	faArrowRightArrowLeft,
	faArrowsLeftRight,
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
	faDocker,
	faDownload,
	faFileImport,
	faFingerprint,
	faFloppyDisk,
	faForward,
	faFunction,
	faGear,
	faGlobe,
	faInfoSquare,
	faKey,
	faLayerGroup,
	faLeaf,
    faLightbulb,
	faLink,
	faListUl,
	faMaximize,
	faMemory,
	faMerge,
	faNetworkWired,
	faNextjs,
	faNodeJs,
	faPaintbrush,
	faPalette,
    faRailway,
	faReact,
	faRecycle,
	faRocket,
	faRotate,
	faRust,
	faScrewdriverWrench,
	faServer,
	faShareNodes,
	faSitemap,
	faSliders,
	faSlidersHSquare,
	faSquareBinary,
	faSquareInfo,
    faSquareRootVariable,
	faSquareSliders,
	faSquareTerminal,
	faTag,
	faTowerBroadcast,
    faUpload,
	faVercel,
	faVialCircleCheck,
} from "@rivet-gg/icons";
import type { DeployOption } from "@/data/deploy-options";
import { deployOptions } from "@/data/deploy-options";
import { integrationGroups } from "@/data/integrations/shared";
import { useCases } from "@/data/use-cases";
import nextjs from "@/images/vendors/next-js.svg";
import type { SidebarItem, Sitemap } from "@/lib/sitemap";

// Goals:
// - Siebar links should advertise the product, collapse any advanced pages away
// - The sidebar should be 1 screen height when collapsed

// Profiles:
// - What does Rivet do?
//	- Does it work for my use cases -> Use Cases
//	- Curious about the technology -> Build with Rivet
// - Just want to jump in
// - People who want to run Open Source

const deployHosts: DeployOption[] = deployOptions;

const integrationSidebarSections: SidebarItem[] = integrationGroups.map(
	({ title: groupTitle, items }) => ({
		title: groupTitle,
		pages: items.map(({ title, href }) => ({
			title,
			href,
		})),
	}),
);

export const sitemap = [

	{
		title: "Overview",
		href: "/docs",
		sidebar: [
			{
				title: "General",
				pages: [
					{
						title: "Overview",
						href: "/docs",
						icon: faSquareInfo,
					},
				]
			},
			{
				title: "Use Cases",
				pages: [
					...useCases.slice(0, 3).map(({ title, href, icon }) => ({
						title,
						href,
						icon,
					})),
					{
						title: "More",
						collapsible: true,
						pages: useCases.slice(3).map(({ title, href, icon }) => ({
							title,
							href,
							icon,
						})),
					},
				],
			},
			{
				title: "Concepts",
				pages: [
					{
						title: "What are Rivet Actors?",
						href: "/docs/actors",
						icon: faSquareInfo,
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
						title: "Data Management",
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
						title: "More",
						icon: faSitemap,
						collapsible: true,
						pages: [
							{
								title: "Testing",
								href: "/docs/actors/testing",
								icon: faVialCircleCheck,
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
							{
								title: "Scaling & Concurrency",
								href: "/docs/actors/scaling",
								//icon: faMaximize,
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
						href: "/docs/actors/clients",
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
								title: "Next.js",
								href: "/docs/clients/next-js",
								icon: faNextjs,
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
						]
					}
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
						title: "Docs for LLMs",
						href: "/docs/general/docs-for-llms",
						// icon: faSquareBinary,
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
		title: "Quickstart",
		href: "/docs/quickstart",
		sidebar: [
			{
				title: "General",
				pages: [
					{
						title: "Overview",
						href: "/docs/quickstart",
						icon: faForward,
					},
				],
			},
			{
				title: "Guides",
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
						title: "Cloudflare Workers",
						href: "/docs/actors/quickstart/cloudflare-workers",
						icon: faCloudflare,
					},
				],
			},
		],
	},
	{
		title: "Integrations",
		href: "/docs/integrations",
		// IMPORTANT: Also update integrations/index.mdx
		sidebar: [
			{
				title: "General",
				pages: [
					{
						title: "Overview",
						href: "/docs/integrations",
						icon: faSquareInfo,
					},
				]
			},
			...integrationSidebarSections,
		],
	},

	{
		title: "Deploy",
		href: "/docs/deploy",
		sidebar: deployHosts.map(({ title, href, icon, badge }) => ({
			title,
			href,
			icon,
			badge,
		})),
	},
	{
		title: "Rivet Cloud",
		href: "/docs/cloud",
		sidebar: [
			{
				title: "Overview",
				href: "/docs/cloud",
				// icon: faSquareInfo,
			},
		],
	},
	{
		title: "Self-Hosting",
		href: "/docs/self-hosting",
		sidebar: [
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
				title: "Actors",
				collapsible: true,
				pages: [
					{
						title: "Connect Backend",
						href: "/docs/self-hosting/connect-backend",
						// icon: faNetworkWired,
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
				],
			},
			{
				title: "Platforms",
				collapsible: true,
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
