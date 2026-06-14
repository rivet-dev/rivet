import type { FaqItem } from './types';

// FAQ content for the Rivet Cloud pricing page. Rendered by PricingPageClient
// and emitted as FAQPage JSON-LD from pages/cloud.astro. Keep numbers in sync
// with the plan and usage pricing data in PricingPageClient.tsx.
export const cloudFaqs: FaqItem[] = [
	{
		question: 'What is an Awake Actor Hour?',
		answerHtml:
			'An Awake Actor Hour is one Rivet Actor running for one hour. Actors automatically <a href="https://rivet.dev/docs/actors/lifecycle/">sleep when idle</a> and wake on demand, so you are only billed for the time your actors are actually awake. Usage is billed by the second and rounded up to the nearest second.',
	},
	{
		question: 'What does the free plan include?',
		answerHtml:
			'The free plan includes 100,000 Awake Actor Hours per month, 5GB of state storage, 5 million writes per month, 200 million reads per month, 100GB of egress, and community support. It is designed for prototyping and small projects.',
	},
	{
		question: 'How does usage pricing work beyond plan limits?',
		answerHtml:
			'Paid plans include a usage allowance, and metered pricing applies beyond it: $0.05 per 1,000 Awake Actor Hours, $0.40 per GB-month of state storage, $0.20 per million reads, $1 per million writes, and $0.15 per GB of egress. Reads and writes refer to persisted actor state, not in-memory operations within an actor. The free plan has fixed monthly limits; upgrade to Hobby to scale with usage pricing.',
	},
	{
		question: 'Do sleeping actors cost money?',
		answerHtml:
			'No. Sleeping actors consume no Awake Actor Hours. While an actor sleeps you only pay for its persisted state storage, so workloads that idle most of the time scale to zero compute cost. The actor wakes automatically when a request, message, or scheduled alarm arrives.',
	},
	{
		question: 'Is Rivet free to self-host?',
		answerHtml:
			'Yes. Rivet is open source under the Apache 2.0 license and free to run on your own infrastructure with no usage limits. It ships as a single Rust binary or Docker image. See the <a href="https://rivet.dev/docs/self-hosting/">self-hosting docs</a> to get started, or use Rivet Cloud if you prefer managed infrastructure.',
	},
	{
		question: 'Can Rivet run in my VPC, on-prem, or air-gapped?',
		answerHtml:
			'Yes. The open-source release runs in your VPC, on-prem, or fully air-gapped with no usage limits, using filesystem or Postgres storage. For production deployments in regulated networks, the Enterprise Edition adds the FoundationDB persistence layer, hardening guidance, and priority support. <a href="https://rivet.dev/sales/">Contact sales</a> for details.',
	},
	{
		question: 'What support is included with each plan?',
		answerHtml:
			'The free plan includes community support through Discord and GitHub. Hobby includes email support, Team includes Slack and email support, and Enterprise includes priority support with an SLA.',
	},
	{
		question: 'Do you offer discounts for startups or high-volume usage?',
		answerHtml:
			'Yes. New Rivet Cloud customers from Y Combinator or a16z Speedrun, including alumni, get 50% off the Team plan for 12 months through our <a href="https://rivet.dev/startups/">startup deal</a>. For high-volume workloads, Enterprise plans include volume pricing; <a href="https://rivet.dev/sales/">contact sales</a> for a custom quote.',
	},
];
