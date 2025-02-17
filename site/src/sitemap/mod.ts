import apiPages from "@/generated/apiPages.json" assert { type: "json" };
import type { Sitemap } from "@/lib/sitemap";
import {
	faActorsBorderless,
	faBlockQuestion,
	faClipboardListCheck,
	faCode,
	faDatabase,
	faExclamationTriangle,
	faFingerprint,
	faFloppyDisk,
	faGlobe,
	faHammer,
	faHeartPulse,
	faListUl,
	faMaximize,
	faNetworkWired,
	faPlay,
	faReact,
	faSquareInfo,
	faSquarePlus,
	faSquareSliders,
	faSync,
	faTag,
	faTowerBroadcast,
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
			{ title: "Overview", href: "/docs", icon: faSquareInfo },
			{
				title: "Getting Started",
				pages: [
					{
						title: "Initial Setup",
						href: "/docs/setup",
						icon: faPlay,
					},
					// {
					//   title: 'Client SDKs',
					//   href: '/docs/client/javascript',
					//   icon: 'toolbox'
					// },
					// {
					//   title: 'JavaScript SDK',
					//   href: '/docs/client/javascript',
					//   icon: 'js'
					// },
					// {
					//   title: 'Client SDKs',
					//   collapsible: true,
					//   pages: [
					//     {
					//       title: 'JavaScript & TypeScript',
					//       href: '/docs/client/javascript'
					//     }
					//     // TODO:
					//     // { title: 'Godot', href: '/docs/godot' },
					//     // { title: 'Unity', href: '/docs/unity' },
					//     // { title: 'Unreal', href: '/docs/unreal' },
					//   ]
					// },
					{
						title: "Actor SDK",
						href: "https://www.npmjs.com/package/@rivet-gg/actor",
						icon: faActorsBorderless,
						external: true,
					},
					// {
					//   title: 'JavaScript SDK',
					//   href: 'https://jsr.io/@rivet-gg/actor-client',
					//   icon: 'js'
					// },
				],
			},
			{
				title: "Client SDKs",
				pages: [
					{
						title: "TypeScript",
						href: "https://www.npmjs.com/package/@rivet-gg/actor-client",
						icon: faTs,
						external: true,
					},
					{
						title: "React",
						href: "/docs/client/react",
						icon: faReact,
					},
				],
			},
			//{
			//  title: 'Use Cases',
			//  pages: [
			//    {
			//      title: 'Collaborative Application',
			//      href: '/use-cases/multiplayer',
			//      icon: 'rotate'
			//    },
			//    {
			//      title: 'Local-First Sync',
			//      href: '/use-cases/local-first',
			//      icon: 'mobile'
			//    },
			//    {
			//      title: 'AI Agents',
			//      href: '/use-cases/ai-agents',
			//      icon: 'robot'
			//    },
			//    {
			//      title: 'Discord Activities',
			//      href: '/use-cases/user-code',
			//      icon: 'alien-8bit'
			//    },
			//    {
			//      title: 'Dedicated Game Servers',
			//      href: '/use-cases/game-servers',
			//      icon: 'gamepad'
			//    },
			//    {
			//      title: 'Run User Code',
			//      href: '/use-cases/user-code',
			//      icon: 'alien-8bit'
			//    },
			//    // { title: 'Batch Jobs', href: '/docs', icon: 'forklift' },
			//    // { title: 'Live Events', href: '/docs', icon: 'calendar' },
			//    { title: 'More', href: '/use-cases', icon: 'ellipsis' }
			//  ]
			//},
			{
				title: "Build with Actors",
				pages: [
					{
						title: "Create & Manage Actors",
						href: "/docs/manage",
						icon: faSquarePlus,
					},
					{
						title: "Remote Procedure Calls",
						href: "/docs/rpc",
						icon: faCode,
					},
					{
						title: "State",
						href: "/docs/state",
						icon: faFloppyDisk,
					},
					{
						title: "Events",
						href: "/docs/events",
						icon: faTowerBroadcast,
					},
					{
						title: "Lifecycle",
						href: "/docs/lifecycle",
						icon: faSync,
					},
					{
						title: "Edge Networking",
						href: "/docs/edge",
						icon: faGlobe,
					},
					{
						title: "More",
						collapsible: true,
						pages: [
							{
								title: "Connections",
								href: "/docs/connections",
								icon: faNetworkWired,
							},
							{
								title: "Authentication",
								href: "/docs/authentication",
								icon: faFingerprint,
							},
							{
								title: "Metadata",
								href: "/docs/metadata",
								icon: faTag,
							},
							{
								title: "External SQL Databases",
								href: "/docs/external-sql",
								icon: faDatabase,
							},
							{
								title: "Logging",
								href: "/docs/logging",
								icon: faListUl,
							},
							{
								title: "Builds",
								href: "/docs/builds",
								icon: faHammer,
							},
							{
								title: "Scaling & Concurrency",
								href: "/docs/scaling",
								icon: faMaximize,
							},
							{
								title: "Fault Tolerance",
								href: "/docs/fault-tolerance",
								icon: faHeartPulse,
							},
							// { title: 'DDoS & Botting Mitigation', href: '/docs', icon: 'shield-halved' },
						],
					},
				],
			},
			{
				title: "Resources",
				pages: [
					// { title: 'Cheatsheet', href: '/docs/cheatsheet', icon: 'file-code' },
					// { title: 'Integrating Exiting Projects', href: '/docs/integrate', icon: 'plug' },
					{
						title: "Configuration",
						href: "/docs/config",
						icon: faSquareSliders,
					},
					{
						title: "Troubleshooting",
						href: "/docs/troubleshooting",
						icon: faClipboardListCheck,
					},
					{
						title: "FAQ",
						href: "/docs/faq",
						icon: faBlockQuestion,
					},
					// { title: 'CLI', href: '/docs/cli', icon: 'square-terminal' },
					// { title: 'Hub', href: '/docs/hub', icon: 'browser' },
					// { title: 'Local Development', href: '/docs/local-development', icon: 'display' },
					// { title: 'Billing', href: '/docs', icon: 'credit-card' },
					// {
					// 	title: 'Low-Level API',
					// 	collapsible: true,
					// 	pages: [
					// 		{ title: 'Containers vs Isolates', href: '/docs', icon: 'box' },
					// 		{ title: 'Tokens', href: '/docs', icon: 'box' },
					// 		{ title: 'Durability', href: '/docs', icon: 'box' },
					// 		{ title: 'Advanced Networking', href: '/docs', icon: 'box' },
					// 	]
					// },
					{
						title: "Self-Hosting",
						collapsible: true,
						pages: [
							{
								title: "Overview",
								href: "/docs/self-hosting",
							},
							{
								title: "Local Dev & Single Container",
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
					// {
					//   title: 'Comparison',
					//   collapsible: true,
					//   pages: [
					//     {
					//       title: 'Kubernetes Jobs',
					//       href: '/compare/kubernetes'
					//     },
					//     {
					//       title: 'Cloudflare Durable Objects',
					//       href: '/compare/cloudflare'
					//     },
					//     { title: 'Firebase', href: '/compare/firebase' },
					//     { title: 'Socket.io', href: '/compare/socket-io' },
					//     { title: 'Redis', href: '/compare/redis' },
					//     {
					//       title: 'Erlang/OTP & Elixir',
					//       href: '/docs/erlang'
					//     }
					//     // { title: 'Supabase Realtime', href: '/docs' },
					//   ]
					// },
					{
						title: "More",
						collapsible: true,
						pages: [
							{
								title: "Available Regions",
								href: "/docs/regions",
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
										title: "Rescheduling",
										href: "/docs/rescheduling",
									},
									{
										title: "Networking",
										href: "/docs/networking",
									},
									{
										title: "Internals",
										collapsible: true,
										pages: [
											{
												title: "Design Decisions",
												href: "/docs/internals/design-decisions",
											},
											{
												title: "Actor Runtime",
												href: "/docs/internals/runtime",
											},
										],
									},
								],
							},
						],
					},
				],
			},
		],
	},
	// {
	//   title: 'Examples',
	//   href: '/examples',
	//   sidebar: [
	//     // TODO: Group by type in sidebar
	//   ]
	// },
	{
		title: "Platform API",
		href: "/docs/api",
		sidebar: apiPages.pages,
	},
] satisfies Sitemap;
