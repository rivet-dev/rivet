import type { CompareEntry } from './types';

// Facts about Temporal verified against temporal.io and docs.temporal.io in
// June 2026: pricing plans and per-action billing, the documented ~100ms
// minimum end-to-end Cloud latency, worker poll architecture with no ingress,
// seven GA SDKs plus a preview Rust SDK, MIT server license, and self-hosting requirements.
// Re-verify pricing before major updates since pricing pages churn.
export const temporal: CompareEntry = {
	slug: 'rivet-vs-temporal',
	competitorName: 'Temporal',
	rivetProductName: 'Rivet',
	title: 'Rivet vs Temporal',
	description:
		'Compare Rivet and Temporal: stateful realtime actors with durable workflows in one runtime versus a dedicated durable execution platform for orchestration.',
	heroSubtitle:
		'Temporal is a durable execution platform for orchestrating backend workflows behind your API. Rivet combines stateful actors, realtime connections, and durable workflows in one open-source runtime that serves traffic directly.',
	rivetSummary:
		'Rivet is an open-source runtime for stateful backends. Rivet Actors hold persistent state, serve HTTP and WebSocket traffic directly, and include durable workflows with steps, timers, and queue waits, so one runtime covers realtime serving and reliable multi-step execution.',
	competitorSummary:
		'Temporal is a durable execution platform built around workflow-as-code. Workflows run on your worker fleet, coordinate through the Temporal Service with full event-history replay, and recover from failures automatically. It is battle-tested at very large scale and orchestrates work behind your API rather than serving user traffic itself.',
	whenToChooseRivet: [
		{
			title: 'Realtime and workflows in one runtime',
			description:
				'When your product needs WebSockets, presence, or per-user sessions alongside durable multi-step logic, without running a separate orchestrator next to your serving layer',
		},
		{
			title: 'Low-latency interactive workloads',
			description:
				"When user-facing requests need to hit stateful code directly instead of round-tripping through a task queue; Temporal's own engineering blog puts minimum end-to-end workflow latency on Temporal Cloud at around 100ms",
		},
		{
			title: 'Long-lived stateful entities',
			description:
				'When you model chatty, long-lived sessions such as agents, documents, or game lobbies that would hit workflow event-history limits and continue-as-new choreography in Temporal',
		},
		{
			title: 'Lightweight self-hosting',
			description:
				'When you want a self-contained engine you can run yourself without operating a multi-service cluster with separate persistence and search stores',
		},
		{
			title: 'Per-entity state and storage',
			description:
				'When each user, document, or agent should own its own persistent state, with KV and an embedded SQLite database co-located with compute',
		},
	],
	whenToChooseCompetitor: [
		{
			title: 'Mature orchestration at massive scale',
			description:
				'When you need a durable execution platform proven on millions to billions of workflow executions in production at large enterprises',
		},
		{
			title: 'Polyglot teams',
			description:
				'When workflows and activities span Go, Java, Python, TypeScript, .NET, PHP, or Ruby, coordinated across languages through one service',
		},
		{
			title: 'Deep replay debugging and audit trails',
			description:
				'When you need a complete event history for every execution, replay-based debugging, and the ability to reset or patch workflows in production',
		},
		{
			title: 'Enterprise reliability requirements',
			description:
				'When you need multi-region or multi-cloud failover with a 99.99% SLA and enterprise controls such as SCIM and private networking',
		},
	],
	featureGroups: [
		{
			title: 'Open Source & Hosting',
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
						status: 'yes',
						text: 'Yes, the Temporal server is MIT-licensed and free to self-host',
					},
					importance:
						'Building your core technology on open-source software ensures portability and flexibility as your needs change',
				},
				{
					feature: 'Self-hosting footprint',
					rivet: {
						status: 'yes',
						text: 'Self-contained engine that runs with Docker Compose or Kubernetes',
					},
					competitor: {
						status: 'partial',
						text: 'Four server services plus a database, with Elasticsearch or OpenSearch recommended beyond small workloads',
					},
					importance:
						'A smaller production footprint means less infrastructure to operate, monitor, and upgrade',
				},
				{
					feature: 'Managed cloud',
					rivet: {
						status: 'yes',
						text: (
							<>
								Rivet Cloud with a free tier.{' '}
								<a href="https://rivet.dev/cloud/">See pricing</a>.
							</>
						),
					},
					competitor: {
						status: 'yes',
						text: 'Temporal Cloud, consumption-based with plan minimums starting at $100 per month; startup credits but no perpetual free tier',
					},
					importance:
						'A managed option lets you start without operating infrastructure and scale later',
				},
			],
		},
		{
			title: 'Serving & Realtime',
			rows: [
				{
					feature: 'Serves user-facing traffic directly',
					rivet: {
						status: 'yes',
						text: 'Actors terminate HTTP and WebSocket connections themselves',
					},
					competitor: {
						status: 'no',
						text: 'Workers poll task queues over outbound connections only; you build a separate API layer that talks to workflows through the Temporal Service',
					},
					importance:
						'Serving traffic from the same runtime that holds state removes an entire tier of gateways and glue code',
				},
				{
					feature: 'WebSockets and realtime events',
					rivet: {
						status: 'yes',
						text: 'Built-in connection handling, event broadcasting, and reconnection logic',
					},
					competitor: {
						status: 'no',
						text: 'No connection layer; signals, queries, and updates round-trip through the service to a polling worker',
					},
					importance:
						'Realtime products need per-connection state and low-latency fanout that task queues are not designed for',
				},
				{
					feature: 'Interactive request latency',
					rivet: {
						status: 'yes',
						text: 'Requests reach in-memory actor state directly',
					},
					competitor: {
						status: 'partial',
						text: "Temporal's own engineering blog puts minimum end-to-end workflow latency at around 100ms on Temporal Cloud and describes tuning techniques to reduce it",
					},
					importance:
						'Latency floors compound across every user interaction in interactive products',
				},
				{
					feature: 'Per-entity state and storage',
					rivet: {
						status: 'yes',
						text: (
							<>
								Each actor owns persistent KV state and an embedded{' '}
								<a href="https://rivet.dev/docs/actors/sqlite/">SQLite database</a>
							</>
						),
					},
					competitor: {
						status: 'partial',
						text: 'Workflow state lives in event history; reads go through queries, and hard history limits require continue-as-new for long-lived entities',
					},
					importance:
						'Long-lived entities such as agents and documents accumulate state that needs direct, queryable storage',
				},
			],
		},
		{
			title: 'Durable Workflows',
			rows: [
				{
					feature: 'Durable multi-step execution',
					rivet: {
						status: 'yes',
						text: (
							<>
								Replay-safe workflows with steps, timers, queue waits, and rollback.{' '}
								<a href="https://rivet.dev/docs/actors/workflows/">Learn more</a>.
							</>
						),
					},
					competitor: {
						status: 'yes',
						text: 'Workflow-as-code with automatic retries, durable timers, and full event-history replay',
					},
					importance:
						'Durable execution lets multi-step operations survive crashes and restarts without manual checkpointing',
				},
				{
					feature: 'Scheduling and cron',
					rivet: {
						status: 'yes',
						text: 'Durable one-shot timers built in with schedule.after and schedule.at; recurring jobs re-arm themselves',
					},
					competitor: {
						status: 'yes',
						text: 'First-class Schedules with backfill, pause, and trigger; each schedule run bills three actions on Temporal Cloud',
					},
					importance:
						'Native scheduling avoids bolting an external cron system onto durable logic',
				},
				{
					feature: 'Code constraints',
					rivet: {
						status: 'partial',
						text: 'Side effects must run inside recorded steps; the rest of the run loop replays automatically',
					},
					competitor: {
						status: 'partial',
						text: 'Workflow code must be fully deterministic; I/O, time, and randomness go through activities or SDK-provided equivalents, and non-determinism errors are a common pitfall',
					},
					importance:
						'Determinism rules shape how much existing code you can reuse and how steep the learning curve is',
				},
				{
					feature: 'AI agent workloads',
					rivet: {
						status: 'yes',
						text: 'Actors hold agent state, stream tokens over WebSockets, and run durable agent loops in one place',
					},
					competitor: {
						status: 'partial',
						text: 'OpenAI Agents SDK and Google ADK integrations run LLM calls as activities; durable response streaming is in public preview',
					},
					importance:
						'Agents need durability for long-running loops and realtime streaming for user experience at the same time',
				},
			],
		},
		{
			title: 'Operations & Ecosystem',
			rows: [
				{
					feature: 'Language support',
					rivet: {
						status: 'partial',
						text: 'TypeScript is the flagship SDK; a Rust SDK is available in preview',
					},
					competitor: {
						status: 'yes',
						text: 'Seven GA SDKs: Go, Java, TypeScript, Python, .NET, PHP, and Ruby, plus Rust in preview',
					},
					importance:
						'SDK breadth matters when teams orchestrate work across several languages',
				},
				{
					feature: 'Replay debugging and audit trail',
					rivet: {
						status: 'partial',
						text: 'Workflow step progress and actor state are inspectable in the dashboard',
					},
					competitor: {
						status: 'yes',
						text: 'Complete event history per execution with local replay, resets, and production debugging tools',
					},
					importance:
						'Audit-grade history makes incident forensics and compliance reviews far easier',
				},
				{
					feature: 'State inspector',
					rivet: {
						status: 'yes',
						text: 'Built-in tools to view and edit live actor state, connections, and RPCs',
					},
					competitor: {
						status: 'partial',
						text: 'Web UI shows event history and workflow status; reading state requires queries',
					},
					importance:
						'Direct visibility into live state speeds up debugging and operations',
				},
				{
					feature: 'Local development',
					rivet: {
						status: 'yes',
						text: 'One dev process; the engine runs alongside your application',
					},
					competitor: {
						status: 'yes',
						text: 'Single-binary dev server via the Temporal CLI with no external dependencies',
					},
					importance:
						'Fast local setup keeps the development loop short for every engineer on the team',
				},
				{
					feature: 'Cost model for chatty workloads',
					rivet: {
						status: 'yes',
						text: 'Rivet Cloud bills primarily on compute (Awake Actor Hours), not per message',
					},
					competitor: {
						status: 'partial',
						text: 'Every workflow start, activity, retry, signal, query, update, and timer bills as an action, starting at $50 per million',
					},
					importance:
						'Realtime sessions send many small messages; per-message billing makes them structurally expensive',
				},
			],
		},
	],
	verdict: [
		'Temporal is the more mature choice for pure backend orchestration. If your workload is multi-step business processes behind an API, with polyglot teams and strict audit requirements, its durable execution model is battle-tested at enormous scale and its replay debugging is unmatched.',
		'Rivet covers durable workflows and the serving layer in one runtime. Choose Rivet when your product is interactive: agents, collaborative apps, multiplayer, or per-user sessions where the same entity that holds state must also serve WebSocket and HTTP traffic at low latency. You get steps, timers, and replay-safe workflows without operating a separate orchestrator or paying per message.',
	],
	migration: {
		heading: 'Evaluating Rivet alongside Temporal?',
		body: 'Our team can help you map workflows to Rivet Actors and decide which workloads belong where. We provide migration assistance, technical guidance, and dedicated support.',
	},
	faq: [
		{
			question: 'Is Rivet an alternative to Temporal?',
			answerHtml:
				'For many workloads, yes. Rivet provides durable, replayable workflows inside Rivet Actors, plus realtime serving that Temporal does not offer. If your workload is pure backend orchestration across many languages, Temporal is a strong choice. If it combines durable logic with stateful realtime serving, Rivet covers both in one runtime. See <a href="https://rivet.dev/docs/actors/workflows/">Rivet workflows</a>.',
		},
		{
			question: 'Does Rivet support durable execution like Temporal?',
			answerHtml:
				'Yes. Rivet workflows persist progress automatically, replay safely after restarts and crashes, and support steps, durable timers, queue waits, and rollback. They run inside Rivet Actors, so durable logic lives next to the state it operates on. See the <a href="https://rivet.dev/docs/actors/workflows/">workflow documentation</a>.',
		},
		{
			question: 'Can Temporal serve WebSockets or user-facing traffic?',
			answerHtml:
				'No. Temporal workers poll task queues over outbound connections and do not terminate HTTP or WebSocket traffic; you build a separate API layer that communicates with workflows through the Temporal Service. Rivet Actors serve HTTP and WebSocket traffic directly from the runtime that holds state.',
		},
		{
			question: 'How does pricing differ between Rivet Cloud and Temporal Cloud?',
			answerHtml:
				'Temporal Cloud bills per action: workflow starts, activities and their retries, signals, queries, updates, and timers all count, with plan minimums starting at $100 per month and no perpetual free tier. Rivet Cloud bills primarily on compute through Awake Actor Hours and includes a free tier, so message-heavy realtime sessions do not multiply costs. See <a href="https://rivet.dev/cloud/">Rivet Cloud pricing</a>.',
		},
		{
			question: 'Is Rivet open source like Temporal?',
			answerHtml:
				'Yes. Rivet is licensed under Apache 2.0 and the Temporal server is MIT-licensed, and both can be self-hosted. Rivet ships as a self-contained engine, while a production Temporal cluster runs four services plus a database and usually Elasticsearch or OpenSearch. The source is at <a href="https://github.com/rivet-dev/rivet">github.com/rivet-dev/rivet</a>.',
		},
		{
			question: 'Which languages do Rivet and Temporal support?',
			answerHtml:
				'Temporal has seven GA SDKs (Go, Java, TypeScript, Python, .NET, PHP, and Ruby), with Rust available in preview. Rivet is TypeScript-first with a Rust SDK in preview. If your team orchestrates workflows across many languages today, Temporal has the broader SDK surface.',
		},
	],
	lastUpdated: '2026-06-11',
	keywords: [
		'temporal alternative',
		'rivet vs temporal',
		'durable execution',
		'durable workflows',
		'workflow orchestration',
		'temporal.io alternative',
	],
};
