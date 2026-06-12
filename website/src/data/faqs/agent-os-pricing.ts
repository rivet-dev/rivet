import type { FaqItem } from './types';

// FAQ content for the agentOS pricing page. Rendered by AgentOSPricingPage
// and emitted as FAQPage JSON-LD from pages/agent-os/pricing.astro.
export const agentOsPricingFaqs: FaqItem[] = [
	{
		question: 'Is agentOS really free?',
		answerHtml:
			'Yes. agentOS is open source under the Apache 2.0 license. You can run it on your own infrastructure at no cost. Rivet Cloud is a paid service for those who want managed infrastructure.',
	},
	{
		question: 'What is the difference between self-hosted and Rivet Cloud?',
		answerHtml:
			'Self-hosted means you run agentOS on your own servers. You handle deployment, scaling, and maintenance. Rivet Cloud is a fully managed service where we handle all of that for you.',
	},
	{
		question: 'Can I switch from self-hosted to Rivet Cloud later?',
		answerHtml:
			'Absolutely. agentOS uses the same API whether you self-host or use Rivet Cloud. You can migrate with minimal code changes.',
	},
	{
		question: 'What support is available for open source users?',
		answerHtml:
			'Open source users can get help through our Discord community and GitHub issues. Enterprise customers receive dedicated support channels with guaranteed response times.',
	},
	{
		question: 'Do you offer volume discounts?',
		answerHtml:
			'Yes. Contact our sales team for custom pricing on high-volume usage or enterprise deployments.',
	},
];
