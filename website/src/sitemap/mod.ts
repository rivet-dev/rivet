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
    faPuzzlePiece,
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
import type { DeployOption } from "@/data/deploy/shared";
import { deployGroups, deployOptions } from "@/data/deploy/shared";
import nextjs from "@/images/vendors/next-js.svg";
import type { SidebarItem, Sitemap } from "@/lib/sitemap";

const deploySidebarSections: SidebarItem[] = deployGroups.map(
	({ title: groupTitle, items }) => ({
		title: groupTitle,
		pages: items.map(({ title, href, icon, badge }) => ({
			title,
			href,
			icon,
			badge,
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
				title: "Quickstart",
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
								title: "Low-Level WebSocket Handler",
								href: "/docs/actors/websocket-handler"
							},
							{
								title: "Low-Level HTTP Handler",
								href: "/docs/actors/request-handler"
							},
							{
								title: "Vanilla HTTP API",
								href: "/docs/actors/http-api"
							},
						],
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
								title: "Destroying",
								href: "/docs/actors/destroy",
								//icon: faTag,
							},
						],
					},
					{
						title: "Design Patterns",
						icon: faLayerGroup,
						href: "/docs/actors/design-patterns",
					},
					{
						title: "More",
						icon: faSitemap,
						collapsible: true,
						pages: [
							{
								title: "Ephemeral Variables",
								href: "/docs/actors/ephemeral-variables",
								//icon: faMemory,
							},
							{
								title: "External SQL",
								href: "/docs/actors/external-sql",
								//icon: faDatabase,
							},
							{
								title: "Logging",
								href: "/docs/general/logging",
								// icon: faListUl,
							},
							{
								title: "Errors",
								href: "/docs/actors/errors"
							},
							{
								title: "Testing",
								href: "/docs/actors/testing",
								// icon: faVialCircleCheck,
							},
							{
								title: "AI & User-Generated Actors",
								href: "/docs/actors/ai-and-user-generated-actors",
							},
							{
								title: "Helper Types",
								href: "/docs/actors/helper-types",
								//icon: faCode,
							},
							{
								title: "CORS",
								href: "/docs/general/cors",
								// icon: faShareNodes,
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
							// {
							// 	title: "Rust",
							// 	href: "/docs/clients/rust",
							// 	icon: faRust,
							// },
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
						title: "API Reference",
						collapsible: true,
						pages: [
							{
								title: "TypeScript API",
								href: "/typedoc",
								external: true
								// icon: faSquareBinary,
							},
							{
								title: "OpenAPI",
								href: "https://github.com/rivet-dev/rivet/tree/main/rivetkit-openapi",
								external: true
								// icon: faSquareBinary,
							},
							{
								title: "AsyncAPI",
								href: "https://github.com/rivet-dev/rivet/tree/main/rivetkit-asyncapi",
								external: true
								// icon: faSquareBinary,
							},
						]
					},
					{
						title: "More",
						collapsible: true,
						pages: [
							{
								title: "Submit Template",
								href: "/docs/meta/submit-template",
							},
						]
					},
					// {
					// 	title: "Architecture",
					// 	href: "/docs/general/architecture",
					// 	// icon: faSquareBinary,
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
	// {
	// 	title: "Integrations",
	// 	href: "/docs/integrations",
	// 	// IMPORTANT: Also update integrations/index.mdx
	// 	sidebar: [
	// 		{
	// 			title: "General",
	// 			pages: [
	// 				{
	// 					title: "Overview",
	// 					href: "/docs/integrations",
	// 					icon: faSquareInfo,
	// 				},
	// 			]
	// 		},
	// 		...integrationSidebarSections,
	// 	],
	// },

	{
		title: "Connect",
		href: "/docs/connect",
		sidebar: [
			{
				title: "General",
				pages: [
					{
						title: "Overview",
						href: "/docs/connect",
						icon: faSquareInfo,
					},
				]
			},
			...deploySidebarSections,
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
						title: "Connect Backend",
						href: "/docs/self-hosting/connect-backend",
						// icon: faNetworkWired,
					},
					{
						title: "Multi-Region",
						href: "/docs/self-hosting/multi-region",
						// icon: faGlobe,
					},
				]
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
						title: "Kubernetes",
						href: "/docs/self-hosting/kubernetes",
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
