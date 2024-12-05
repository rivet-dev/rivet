import { Sitemap, SiteTab } from '@/lib/sitemap';
// import { advanced, common, developingModules, platforms, usingModules } from '@/sitemap/common';

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
		title: 'Documentation',
		href: '/docs',
		sidebar: [
			{ title: 'Overview', href: '/docs', icon: 'square-info' },
			{
				title: 'Getting Started',
				pages: [
					{ title: 'Intro to Rivet', href: '/docs/introduction', icon: 'book' },
					{ title: 'Initial Setup', href: '/docs/setup', icon: 'play' },
					{
						// title: 'Languages & Game Engines',
						title: 'Languages',
						collapsible: true,
						pages: [
							{ title: 'JavaScript & TypeScript', href: '/docs/client/javascript' },
							// TODO:
							// { title: 'Godot', href: '/docs/godot' },
							// { title: 'Unity', href: '/docs/unity' },
							// { title: 'Unreal', href: '/docs/unreal' },
						]
					},
				]
			},
			{
				title: 'Use Cases',
				pages: [
					{ title: 'Multiplayer Tools', href: '/use-cases/multiplayer', icon: 'rotate' },
					{ title: 'Local-First Apps', href: '/use-cases/local-first', icon: 'mobile' },
					{ title: 'AI Agents', href: '/use-cases/ai-agents', icon: 'robot' },
					{ title: 'Run User Code', href: '/use-cases/user-code', icon: 'alien-8bit' },
					{ title: 'Dedicated Game Servers', href: '/use-cases/game-servers', icon: 'gamepad' },
					// { title: 'Batch Jobs', href: '/docs', icon: 'forklift' },
					// { title: 'Live Events', href: '/docs', icon: 'calendar' },
					{ title: 'More', href: '/use-cases', icon: 'ellipsis' },
				]
			},
			{
				title: 'Build with Rivet',
				pages: [
					{ title: 'What Are Actors?', href: '/docs/actors', icon: 'block-question' },
					{ title: 'Functions', href: '/docs/functions', icon: 'code' },
					{ title: 'State', href: '/docs/state', icon: 'floppy-disk' },
					{ title: 'Events', href: '/docs/events', icon: 'tower-broadcast' },
					{ title: 'Scaling & Concurrency', href: '/docs/scaling', icon: 'maximize' },
					{ title: 'Edge Networking', href: '/docs/edge', icon: 'globe' },
					{
						title: 'Concepts',
						collapsible: true,
						pages: [
							{ title: 'Lifecycle', href: '/docs/lifecycle', icon: 'sync' },
							{ title: 'Connections', href: '/docs/connections', icon: 'network-wired' },
							{ title: 'Authentication', href: '/docs/authentication', icon: 'fingerprint' },
							{ title: 'Fault Tolerance', href: '/docs/fault-tolerance', icon: 'heart-pulse' },
							// { title: 'DDoS & Botting Mitigation', href: '/docs', icon: 'shield-halved' },
						]
					},
				]
			},
			{
				title: 'Resources',
				pages: [
					// { title: 'Cheatsheet', href: '/docs/cheatsheet', icon: 'file-code' },
					// { title: 'Integrating Exiting Projects', href: '/docs/integrate', icon: 'plug' },
					{ title: 'Configuration', href: '/docs/config', icon: 'square-sliders' },
					{ title: 'Available Regions', href: '/docs/regions', icon: 'globe' },
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
						title: 'Self-Hosting',
						collapsible: true,
						pages: [
							{ title: 'Introduction', href: '/docs/self-hosting' },
							{ title: 'Docker Compose', href: '/docs/self-hosting/docker-compose' },
							{ title: 'Manual Deployment', href: '/docs/self-hosting/manual-deployment' },
							{ title: 'Server Config', href: '/docs/self-hosting/server-config' },
							{ title: 'Client Config', href: '/docs/self-hosting/client-config' }
						]
					},
					{
						title: 'Comparison',
						collapsible: true,
						pages: [
							{ title: 'Kubernetes Jobs', href: '/compare/kubernetes' },
							{ title: 'Cloudflare Durable Objects', href: '/compare/cloudflare' },
							{ title: 'Firebase', href: '/compare/firebase' },
							{ title: 'Socket.io', href: '/compare/socket-io' },
							{ title: 'Redis', href: '/compare/redis' },
							{ title: 'Erlang/OTP & Elixir', href: '/docs/erlang' },
							// { title: 'Supabase Realtime', href: '/docs' },
						]
					},
					{
						title: 'Advanced',
						collapsible: true,
						pages: [
							{ title: 'Rescheduling', href: '/docs/actor-internals/rescheduling' },
							{ title: 'Networking', href: '/docs/actor-internals/networking' }
						]
					},
				]
			},
		]
	},
	{
		title: 'Examples',
		href: '/examples',
		sidebar: [
			// TODO: Group by type in sidebar
		]
	},
	{
		title: 'Actors SDK',
		href: '/docs/examples',
		sidebar: [
		]
	},
	{
		title: 'Platform API',
		href: '/docs/examples',
		sidebar: [
		]
	},
] satisfies Sitemap;
