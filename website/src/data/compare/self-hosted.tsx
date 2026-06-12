import { faCloud } from '@rivet-gg/icons';
import type { CompareEntry } from './types';

// This entry compares self-hosted Rivet against managed platforms as a
// category, primarily Temporal Cloud and Cloudflare Durable Objects. Every
// competitor claim here reuses a fact already vetted in temporal.tsx or
// cloudflare-durable-objects.tsx; do not add new competitor claims without
// verifying them in those entries first. Rivet self-hosting facts come from
// the self-hosting docs: self-contained engine, Docker Compose, Kubernetes,
// and air-gapped deployments.
export const selfHosted: CompareEntry = {
	slug: 'self-hosted-vs-managed-platforms',
	competitorName: 'Managed Platforms',
	rivetProductName: 'Self-Hosted Rivet',
	competitorIcon: faCloud,
	title: 'Self-Hosted Rivet vs Managed Platforms',
	description:
		'Compare self-hosted Rivet with managed platforms: a self-contained engine on Docker Compose or Kubernetes versus multi-service clusters and Cloudflare lock-in.',
	heroSubtitle:
		'Managed platforms run your stateful workloads on infrastructure you do not control. Rivet is an open-source platform you can run yourself: a self-contained engine that deploys with Docker Compose or Kubernetes, inside the boundary your existing controls already cover.',
	rivetSummary:
		'Rivet is an open-source platform for stateful backends that runs entirely on your own infrastructure. The engine is self-contained: deploy it as a single container, with Docker Compose, or on Kubernetes, and your actors, durable workflows, and realtime connections stay inside your own network with no usage metering.',
	competitorSummary:
		"Managed platforms such as Temporal Cloud and Cloudflare Durable Objects run the control plane, and often your code, on vendor-operated infrastructure. Temporal can also be self-hosted as a multi-service cluster, while Durable Objects run only on Cloudflare's network. In exchange for control, someone else operates, scales, and upgrades the platform.",
	whenToChooseRivet: [
		{
			title: 'Run the platform yourself',
			description:
				'When you want the entire platform on infrastructure you control: a self-contained engine that deploys as one container, with Docker Compose, or on Kubernetes',
		},
		{
			title: 'Deploy inside your compliance boundary',
			description:
				'When workloads must run inside the network boundary your existing controls already cover: your VPC, your cloud account, or an air-gapped environment, with full control over data residency',
		},
		{
			title: 'Predictable cost for chatty agent workloads',
			description:
				'When agents and realtime sessions send many small messages; self-hosted Rivet has no usage metering, and Rivet Cloud bills primarily on compute (Awake Actor Hours), not per action',
		},
		{
			title: 'No vendor lock-in',
			description:
				'When you want the same Apache 2.0 codebase running self-hosted today and on Rivet Cloud tomorrow, without rewriting application code',
		},
		{
			title: 'Small operational footprint',
			description:
				'When you want to self-host without operating a multi-service cluster with separate persistence and search stores',
		},
	],
	whenToChooseCompetitor: [
		{
			title: 'No infrastructure to operate',
			description:
				'When you want the vendor to provision, scale, upgrade, and patch the platform so your team ships application code only',
		},
		{
			title: 'Enterprise reliability from the vendor',
			description:
				"When you want guarantees such as Temporal Cloud's multi-region or multi-cloud failover with a 99.99% SLA and enterprise controls such as SCIM and private networking",
		},
		{
			title: 'Global edge by default',
			description:
				"When you want stateful compute on Cloudflare's global edge network without managing regions yourself",
		},
		{
			title: 'Already committed to a vendor ecosystem',
			description:
				'When your team is already building on Cloudflare Workers or Temporal Cloud and wants to stay within that platform',
		},
	],
	featureGroups: [
		{
			title: 'Hosting & Control',
			rows: [
				{
					feature: 'Runs on your own infrastructure',
					rivet: {
						status: 'yes',
						text: (
							<>
								Self-contained engine deploys as one container, with Docker Compose, or
								on Kubernetes.{' '}
								<a href="https://rivet.dev/docs/self-hosting">Self-hosting docs</a>.
							</>
						),
					},
					competitor: {
						status: 'partial',
						text: "Temporal can be self-hosted; Cloudflare Durable Objects are locked to Cloudflare's infrastructure and cannot run anywhere else",
					},
					importance:
						'Owning the platform keeps deployment, data, and upgrades under your control as requirements change',
				},
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
						text: "The Temporal server is MIT-licensed; Cloudflare's workerd runtime is Apache 2.0, but the Durable Objects platform (control plane, storage, distribution) is proprietary",
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
						text: 'A production Temporal cluster runs four server services plus a database, with Elasticsearch or OpenSearch recommended beyond small workloads; Durable Objects have no self-hosted option',
					},
					importance:
						'A smaller production footprint means less infrastructure to operate, monitor, and upgrade',
				},
				{
					feature: 'Managed option on the same code',
					rivet: {
						status: 'yes',
						text: (
							<>
								Rivet Cloud runs the same open-source engine with a free tier, so
								application code does not change.{' '}
								<a href="https://rivet.dev/cloud/">See pricing</a>.
							</>
						),
					},
					competitor: {
						status: 'yes',
						text: "Managed is the primary mode: Temporal Cloud is consumption-based with plan minimums starting at $100 per month, and Durable Objects are part of Cloudflare's platform",
					},
					importance:
						'Moving between self-hosted and managed without a rewrite keeps your options open',
				},
			],
		},
		{
			title: 'Cost',
			rows: [
				{
					feature: 'Cost model for chatty agent workloads',
					rivet: {
						status: 'yes',
						text: 'Self-hosted Rivet has no usage metering; Rivet Cloud bills primarily on compute (Awake Actor Hours), not per action',
					},
					competitor: {
						status: 'partial',
						text: 'Temporal Cloud bills every workflow start, activity, retry, signal, query, update, and timer as an action, starting at $50 per million',
					},
					importance:
						'Agents and realtime sessions send many small messages; per-action billing makes them structurally expensive',
				},
				{
					feature: 'Free tier and minimums',
					rivet: {
						status: 'yes',
						text: 'Self-hosting is free under Apache 2.0; Rivet Cloud includes a free tier',
					},
					competitor: {
						status: 'partial',
						text: 'Temporal Cloud has plan minimums starting at $100 per month, with startup credits but no perpetual free tier',
					},
					importance:
						'Floors and minimums decide what experimentation and small workloads cost',
				},
			],
		},
		{
			title: 'Compliance & Data Control',
			rows: [
				{
					feature: 'Deploy inside your compliance boundary',
					rivet: {
						status: 'yes',
						text: 'Runs inside the boundary your existing controls already cover: your VPC, your cloud account, or an air-gapped environment',
					},
					competitor: {
						status: 'partial',
						text: 'Data and execution live on vendor-operated infrastructure; Temporal Cloud offers enterprise controls such as SCIM and private networking',
					},
					importance:
						'Deploying inside infrastructure your audits already cover avoids scoping a new vendor into your compliance program',
				},
				{
					feature: 'Data sovereignty and network isolation',
					rivet: {
						status: 'yes',
						text: 'Full control over data residency and network isolation within your VPC',
					},
					competitor: {
						status: 'partial',
						text: 'Cloudflare jurisdictions restrict where Durable Objects run and store data (EU, FedRAMP), but there is no VPC or network isolation',
					},
					importance:
						'Data sovereignty ensures compliance with data governance requirements and maintains complete network isolation',
				},
				{
					feature: 'Integrates with existing monitoring',
					rivet: {
						status: 'yes',
						text: 'Works with your existing observability stack',
					},
					competitor: {
						status: 'partial',
						text: "Durable Objects monitoring is mostly Cloudflare-specific; Temporal's web UI shows event history and workflow status",
					},
					importance:
						'Integration with existing monitoring reduces operational overhead',
				},
			],
		},
		{
			title: 'Operations',
			rows: [
				{
					feature: 'Works with existing deploy processes',
					rivet: {
						status: 'yes',
						text: 'Import the library and deploy with your existing CI/CD',
					},
					competitor: {
						status: 'partial',
						text: 'Durable Objects require a Cloudflare-specific deployment process; Temporal workers deploy on your own fleet but coordinate through the Temporal Service',
					},
					importance:
						'Keeping your existing deployment process reduces complexity and learning curve',
				},
				{
					feature: 'Local development',
					rivet: {
						status: 'yes',
						text: 'One dev process; the engine runs alongside your application',
					},
					competitor: {
						status: 'partial',
						text: 'Temporal has a single-binary dev server via the Temporal CLI; Durable Objects have no Docker Compose compatibility',
					},
					importance:
						'Fast local setup keeps the development loop short for every engineer on the team',
				},
			],
		},
	],
	verdict: [
		"Managed platforms are the right call when you do not want to operate infrastructure at all. Temporal Cloud brings enterprise SLAs and someone else carries the pager, and Durable Objects give you stateful compute on Cloudflare's edge with zero servers to run.",
		'Choose self-hosted Rivet when control is the requirement. The engine is self-contained and deploys with Docker Compose or Kubernetes inside the boundary your existing controls already cover, the whole platform is Apache 2.0, and chatty agent workloads do not pay per action. If you later want managed, the same code runs on Rivet Cloud unchanged.',
	],
	migration: {
		heading: 'Planning a self-hosted deployment?',
		body: 'Our team can help you size the engine, pick a storage backend, and plan the rollout on Docker Compose or Kubernetes. We provide migration assistance, technical guidance, and dedicated support.',
	},
	faq: [
		{
			question: 'Can Rivet be fully self-hosted?',
			answerHtml:
				'Yes. Rivet is open source under Apache 2.0 and the engine is self-contained: run it as a single container, with Docker Compose, or on Kubernetes. Air-gapped deployments are supported. See the <a href="https://rivet.dev/docs/self-hosting">self-hosting documentation</a>.',
		},
		{
			question: 'How does self-hosting Rivet compare to self-hosting Temporal?',
			answerHtml:
				'Rivet ships as a self-contained engine that runs with Docker Compose or Kubernetes. A production Temporal cluster runs four server services plus a database, with Elasticsearch or OpenSearch recommended beyond small workloads. Both are free to self-host under open-source licenses.',
		},
		{
			question: 'Can Cloudflare Durable Objects be self-hosted?',
			answerHtml:
				"No. The workerd runtime is open source under Apache 2.0, but the Durable Objects platform (control plane, storage, and distribution) is proprietary and runs only on Cloudflare's infrastructure.",
		},
		{
			question: 'What does self-hosted Rivet cost?',
			answerHtml:
				'The software is free under Apache 2.0; you pay only for the infrastructure you run it on, with no per-action or per-message metering. If you prefer managed, Rivet Cloud bills primarily on compute (Awake Actor Hours) and includes a free tier. See <a href="https://rivet.dev/cloud/">Rivet Cloud pricing</a>.',
		},
		{
			question: 'Does self-hosting Rivet help with compliance?',
			answerHtml:
				'It keeps workloads inside the boundary your existing controls already cover. You control data residency, network isolation, and the full stack within your VPC, your cloud account, or an air-gapped environment, so compliance reviews scope your infrastructure rather than a new vendor.',
		},
		{
			question: 'Can I move between self-hosted Rivet and Rivet Cloud?',
			answerHtml:
				'Yes. Rivet Cloud runs the same open-source engine, so application code is unchanged in either direction. Start self-hosted and move to managed, or the reverse, without a rewrite.',
		},
	],
	lastUpdated: '2026-06-12',
	keywords: [
		'self-hosted actors',
		'self-host rivet',
		'temporal self-hosted',
		'cloudflare durable objects self-hosted',
		'managed platform alternative',
		'self-hosted durable execution',
	],
};
