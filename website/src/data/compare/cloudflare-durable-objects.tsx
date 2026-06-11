import { faCloudflare } from '@rivet-gg/icons';
import type { CompareEntry } from './types';

export const cloudflareDurableObjects: CompareEntry = {
	slug: 'rivet-vs-cloudflare-durable-objects',
	competitorName: 'Cloudflare Durable Objects',
	rivetProductName: 'Rivet Actors',
	competitorIcon: faCloudflare,
	title: 'Rivet vs Cloudflare Durable Objects',
	description:
		'Compare Rivet Actors and Cloudflare Durable Objects: open-source stateful actors on your own infrastructure versus stateful compute tied to Cloudflare.',
	heroSubtitle:
		'Cloudflare Durable Objects provide stateful serverless computing with vendor lock-in. Rivet Actors give you the same capabilities as an open-source library that works with your existing infrastructure and technology stack.',
	rivetSummary:
		'Rivet Actors bring the actor model to your existing infrastructure as an open-source library. Build stateful, distributed applications with your preferred technology stack, deployed on your own infrastructure.',
	competitorSummary:
		"Cloudflare Durable Objects provide stateful serverless computing that runs on Cloudflare's global edge network. Built on Cloudflare's platform, Durable Objects offer strong consistency and state persistence for applications built on the Workers runtime.",
	whenToChooseRivet: [
		{
			title: 'Developer-friendly experience',
			description:
				'When you want an intuitive platform with high-quality documentation, mature local development experience, and in-depth observability into your workloads',
		},
		{
			title: 'Works with your existing infrastructure',
			description:
				'When you want to use actors with your existing deployment process on Kubernetes, AWS, VPS, or any infrastructure',
		},
		{
			title: 'Technology flexibility',
			description:
				'When you want to use your existing frameworks and libraries without platform-specific constraints',
		},
		{
			title: 'Provides monitoring and observability',
			description:
				'When you need built-in monitoring for actors that integrates with your existing observability stack',
		},
		{
			title: 'Rich ecosystem of integrations',
			description:
				'When you want a comprehensive ecosystem with ready-to-use integrations for popular frameworks and tools',
		},
	],
	whenToChooseCompetitor: [
		{
			title: 'Already using Cloudflare ecosystem',
			description:
				"When you're already committed to Cloudflare Workers and want stateful capabilities",
		},
		{
			title: 'Building on the Workers runtime',
			description:
				'When your team is comfortable working within the Workers runtime, with JavaScript/TypeScript, Python in beta, or Rust via workers-rs, and nodejs_compat covers the npm packages you need',
		},
		{
			title: "Don't mind platform constraints",
			description:
				"When you're comfortable with Cloudflare's deployment process, monitoring limitations, and vendor lock-in",
		},
		{
			title: 'Prefer low-level primitives',
			description:
				"When you want raw primitives and don't need a rich ecosystem of framework integrations",
		},
	],
	featureGroups: [
		{
			title: 'Open Source',
			rows: [
				{
					feature: 'Open-source',
					rivet: {
						status: 'yes',
						text: (
							<>
								Yes, Rivet is open-source with the Apache 2.0 license.{' '}
								<a href="https://github.com/rivet-dev/rivet">View on GitHub</a>.
							</>
						),
					},
					competitor: {
						status: 'partial',
						text: 'The workerd runtime is open source under Apache 2.0, but the Durable Objects platform (control plane, storage, distribution) is proprietary',
					},
					importance:
						'Building your core technology on open-source software is vital to ensure portability and flexibility as your needs change',
				},
			],
		},
		{
			title: 'Infrastructure',
			rows: [
				{
					feature: 'Works with existing infrastructure',
					rivet: {
						status: 'yes',
						text: 'Deploy actors on Kubernetes, AWS, VPS, or any infrastructure',
					},
					competitor: {
						status: 'no',
						text: "Locked to Cloudflare's infrastructure",
					},
					importance:
						'Using your existing infrastructure avoids vendor lock-in and integrates with your current setup',
				},
				{
					feature: 'Data sovereignty and VPC isolation',
					rivet: {
						status: 'yes',
						text: 'Full control over data residency and network isolation within your VPC',
					},
					competitor: {
						status: 'no',
						text: "Data processed on Cloudflare's global network with limited control",
					},
					importance:
						'Data sovereignty ensures compliance with data governance requirements and maintains complete network isolation',
				},
				{
					feature: 'Works with existing deploy processes',
					rivet: {
						status: 'yes',
						text: 'Import the library and deploy with your existing CI/CD',
					},
					competitor: {
						status: 'no',
						text: 'Requires Cloudflare-specific deployment process',
					},
					importance:
						'Keeping your existing deployment process reduces complexity and learning curve',
				},
				{
					feature: 'Technology flexibility',
					rivet: {
						status: 'yes',
						text: 'Works with your existing technology stack and frameworks',
					},
					competitor: {
						status: 'partial',
						text: 'Limited to Workers runtime languages (JavaScript/TypeScript, Python in beta, Rust); nodejs_compat covers many npm packages but not native dependencies',
					},
					importance:
						'Technology flexibility lets you use your existing skills and codebase',
				},
				{
					feature: 'Integrates with existing monitoring',
					rivet: {
						status: 'yes',
						text: 'Works with your existing observability stack',
					},
					competitor: {
						status: 'partial',
						text: 'Limited monitoring options, mostly Cloudflare-specific',
					},
					importance:
						'Integration with existing monitoring reduces operational overhead',
				},
			],
		},
		{
			title: 'Runtime',
			rows: [
				{
					feature: 'Actor support',
					rivet: {
						status: 'yes',
						text: 'First-class actor model with Rivet Actors library',
					},
					competitor: {
						status: 'yes',
						text: 'Durable Objects for stateful workloads',
					},
					importance:
						'Actor model enables scalable stateful applications with state persistence and realtime communication',
				},
				{
					feature: 'KV Persistence',
					rivet: {
						status: 'yes',
						text: 'Built-in KV storage for actors',
					},
					competitor: {
						status: 'yes',
						text: 'KV supported for Durable Objects',
					},
					importance:
						'Key-value storage enables persistent state without external dependencies',
				},
				{
					feature: 'SQLite Persistence',
					rivet: {
						status: 'yes',
						text: (
							<>
								Built-in SQLite database per actor with raw SQL and Drizzle support.{' '}
								<a href="https://rivet.dev/docs/actors/sqlite">Learn more</a>.
							</>
						),
					},
					competitor: {
						status: 'yes',
						text: 'SQLite supported for Durable Objects',
					},
					importance:
						'SQLite provides relational database capabilities for complex data models',
				},
				{
					feature: 'Memory limits',
					rivet: {
						status: 'yes',
						text: 'No platform memory cap; memory is bounded by the compute you run actors on',
					},
					competitor: {
						status: 'partial',
						text: '128MB limit for Durable Objects',
					},
					importance:
						'Higher memory limits allow more complex stateful applications',
				},
				{
					feature: 'Automatic connection handling',
					rivet: {
						status: 'yes',
						text: 'Optionally provides abstraction over HTTP, WebSockets, and SSE with intelligent failure and reconnection handling',
					},
					competitor: {
						status: 'no',
						text: 'Requires low-level implementation of connection management',
					},
					importance:
						'Automatic connection handling reduces development time and improves reliability',
				},
				{
					feature: 'Event broadcasting',
					rivet: {
						status: 'yes',
						text: 'Built-in event broadcasting to specific connections or all actors',
					},
					competitor: {
						status: 'partial',
						text: 'Requires hand-rolling fanout over the WebSocket API or adopting a framework like PartyServer',
					},
					importance:
						'Native event system enables real-time features with minimal setup',
				},
				{
					feature: 'Built-in scheduling',
					rivet: {
						status: 'yes',
						text: 'Powerful built-in scheduling system',
					},
					competitor: {
						status: 'partial',
						text: 'Requires boilerplate on top of Alarms API',
					},
					importance:
						'Native scheduling reduces complexity and improves reliability for time-based operations',
				},
				{
					feature: 'Testing support',
					rivet: {
						status: 'yes',
						text: 'Full Vitest support with mocking and fake timers',
					},
					competitor: {
						status: 'partial',
						text: 'Limited Vitest support due to custom runtime constraints',
					},
					importance:
						'Comprehensive testing capabilities ensure code quality and reliability',
				},
				{
					feature: 'Customizable actor lifecycle',
					rivet: {
						status: 'yes',
						text: 'Flexible draining mechanism with configurable lifecycle management',
					},
					competitor: {
						status: 'partial',
						text: '60s grace period',
					},
					importance:
						'Customizable lifecycle management allows for graceful state transfers and prevents data loss',
				},
				{
					feature: 'Control over actor upgrades',
					rivet: {
						status: 'yes',
						text: 'Full control based on your existing rollout mechanisms',
					},
					competitor: {
						status: 'no',
						text: 'Only allows controlling gradual deployment percentages, not specific Durable Object versions',
					},
					importance:
						"Controlled upgrades ensure smooth transitions without service disruption tailored to your application's architecture",
				},
				{
					feature: 'Actor creation with input data',
					rivet: {
						status: 'yes',
						text: 'Pass initialization data when creating actors',
					},
					competitor: {
						status: 'no',
						text: 'Cannot pass input data during Durable Object creation',
					},
					importance:
						'Ability to initialize actors with data simplifies setup and reduces boilerplate',
				},
				{
					feature: 'Actor shutdown control',
					rivet: {
						status: 'yes',
						text: 'Clean shutdown API for actors',
					},
					competitor: {
						status: 'partial',
						text: 'Requires deleteAll with custom logic and error-prone boilerplate',
					},
					importance:
						'Proper shutdown control ensures graceful cleanup and prevents resource leaks',
				},
				{
					feature: 'Monitoring',
					rivet: {
						status: 'yes',
						text: 'Built-in monitoring for development and production',
					},
					competitor: {
						status: 'partial',
						text: 'Per-namespace Durable Objects metrics and GraphQL analytics in the Cloudflare dashboard',
					},
					importance:
						'Integrated monitoring simplifies operations and debugging',
				},
				{
					feature: 'Logging',
					rivet: {
						status: 'yes',
						text: 'Supports your existing logging infrastructure',
					},
					competitor: {
						status: 'yes',
						text: 'Durable Object logs are available through Workers Logs, wrangler tail, and Logpush',
					},
					importance:
						'Built-in logging reduces setup time and operational complexity',
				},
				{
					feature: 'Metadata access',
					rivet: {
						status: 'yes',
						text: 'Built-in metadata API',
					},
					competitor: {
						status: 'no',
						text: 'Custom implementation required',
					},
					importance:
						'Direct access to metadata such as tags, region, and more simplifies management and deployment',
				},
			],
		},
		{
			title: 'Developer Tooling',
			rows: [
				{
					feature: 'State inspector',
					rivet: {
						status: 'yes',
						text: 'Built-in tools to inspect and modify actor state',
					},
					competitor: {
						status: 'partial',
						text: 'Data Studio can view and edit SQLite-backed Durable Object storage from the dashboard; in-memory state is not inspectable',
					},
					importance:
						'Ability to view & edit actor state in real time simplifies debugging and management',
				},
				{
					feature: 'RPC debugger',
					rivet: {
						status: 'yes',
						text: 'Interactive RPC testing tools for actors',
					},
					competitor: {
						status: 'no',
						text: 'No built-in RPC debugging',
					},
					importance:
						'Ability to test remote procedure calls to actors accelerates development and troubleshooting',
				},
				{
					feature: 'Connection inspector',
					rivet: {
						status: 'yes',
						text: 'Real-time monitoring of actor connections',
					},
					competitor: {
						status: 'no',
						text: 'No connection visualization tools',
					},
					importance:
						'Visibility into active connections helps diagnose client-side issues and monitor usage patterns',
				},
				{
					feature: 'Actor listing and management',
					rivet: {
						status: 'yes',
						text: 'Browse and manage active actors with full interaction capabilities',
					},
					competitor: {
						status: 'partial',
						text: 'Can list Durable Objects but cannot interact with them',
					},
					importance:
						'Being able to list and interact with live actors enables debugging and operational management',
				},
			],
		},
		{
			title: 'Development Experience',
			rows: [
				{
					feature: 'Documentation',
					rivet: {
						status: 'yes',
						text: 'Comprehensive, developer-focused documentation',
					},
					competitor: {
						status: 'partial',
						text: 'Fragmented and difficult to understand documentation',
					},
					importance:
						'Clear documentation accelerates learning and implementation',
				},
				{
					feature: 'Compatible with Docker Compose',
					rivet: {
						status: 'yes',
						text: 'Seamless integration with Docker Compose for local development',
					},
					competitor: {
						status: 'no',
						text: 'No Docker Compose compatibility',
					},
					importance:
						'Integration with Docker Compose enables use with your existing development workflows and tools',
				},
			],
		},
	],
	verdict: [
		'While Cloudflare Durable Objects provide stateful serverless computing with vendor lock-in, Rivet Actors offer the same actor model capabilities as an open-source library that works with your existing infrastructure. Choose Rivet Actors when you want the power of actors without changing your deployment process, technology stack, or being locked into a specific platform.',
	],
	migration: {
		heading: 'Migrating from Cloudflare Durable Objects?',
		body: 'Our team can help make the transition smooth and seamless. We provide migration assistance, technical guidance, and dedicated support.',
	},
	faq: [
		{
			question: 'Is Rivet an alternative to Cloudflare Durable Objects?',
			answerHtml:
				'Yes. Rivet Actors provide the same stateful actor model as Durable Objects, including persistent state, WebSockets, and scheduling, as an open-source platform that runs on Rivet Cloud or your own infrastructure. See the <a href="https://rivet.dev/docs/actors">Rivet Actors documentation</a> for details.',
		},
		{
			question: 'Can I run Rivet Actors outside of a single cloud provider?',
			answerHtml:
				'Yes. Rivet Actors run on Rivet Cloud or self-hosted on Kubernetes, AWS, a VPS, or any infrastructure that runs containers. Your application code stays the same in either case, so you are not locked to one provider.',
		},
		{
			question: 'Do Rivet Actors support SQLite storage like Durable Objects?',
			answerHtml:
				'Yes. Each Rivet Actor can use built-in key-value state or its own embedded SQLite database, with raw SQL or Drizzle ORM. See the <a href="https://rivet.dev/docs/actors/sqlite">SQLite documentation</a>.',
		},
		{
			question: 'Is Rivet open source?',
			answerHtml:
				'Yes. Rivet is open source under the Apache 2.0 license. The source code is available at <a href="https://github.com/rivet-dev/rivet">github.com/rivet-dev/rivet</a>.',
		},
		{
			question: 'How hard is it to migrate from Cloudflare Durable Objects to Rivet?',
			answerHtml:
				'Durable Objects and Rivet Actors share the same core concepts, so most migrations map each Durable Object class to a Rivet Actor with state, actions, and scheduling. Our team also provides hands-on migration assistance. <a href="https://rivet.dev/talk-to-an-engineer">Talk to an engineer</a> to plan a migration.',
		},
		{
			question: 'Do Rivet Actors support WebSockets and realtime connections?',
			answerHtml:
				'Yes. Rivet Actors include built-in connection handling for HTTP, WebSockets, and server-sent events, plus event broadcasting to connected clients with automatic reconnection handling.',
		},
	],
	lastUpdated: '2026-06-11',
	keywords: [
		'cloudflare durable objects alternative',
		'rivet vs cloudflare',
		'durable objects',
		'cloudflare workers alternative',
		'stateful serverless',
		'actor model',
	],
};
