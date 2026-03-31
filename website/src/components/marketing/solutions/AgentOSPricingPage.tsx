'use client';

import { useState } from 'react';
import { motion } from 'framer-motion';
import {
	ArrowRight,
	Check,
	Server,
	Cloud,
	Headphones,
	Terminal,
	Copy,
} from 'lucide-react';

const pricingTiers = [
	{
		name: 'Free',
		description: 'Run agentOS anywhere. Free forever.',
		price: 'Free',
		priceSuffix: 'Apache 2.0',
		icon: Server,
		cta: 'npm install rivetkit',
		ctaHref: '',
		copyCommand: 'npm install rivetkit',
		highlight: false,
		features: [
			{ text: 'Full agentOS runtime', included: true },
			{ text: 'Unlimited agents', included: true },
			{ text: 'WebAssembly + V8 isolation', included: true },
			{ text: 'File system mounting (S3, local, etc.)', included: true },
			{ text: 'Tool registry', included: true },
			{ text: 'Cron, webhooks, queues', included: true },
			{ text: 'Network security controls', included: true },
			{ text: 'Community support (Discord, GitHub)', included: true },
		],
	},
	{
		name: 'Enterprise',
		description: 'On-premise deployment with dedicated support.',
		price: 'Custom',
		priceSuffix: 'contact sales',
		icon: Server,
		cta: 'Contact Sales',
		ctaHref: '/sales',
		highlight: false,
		features: [
			{ text: 'On-premise deployment', included: true },
			{ text: 'Air-gapped environments', included: true },
			{ text: 'Custom SLAs', included: true },
			{ text: 'Priority support (Slack)', included: true },
			{ text: 'Custom integrations', included: true },
			{ text: 'Security reviews & compliance', included: true },
		],
	},
];

const CopyButton = ({ command, highlight = false }: { command: string; highlight?: boolean }) => {
	const [copied, setCopied] = useState(false);

	const handleCopy = async () => {
		try {
			await navigator.clipboard.writeText(command);
			setCopied(true);
			setTimeout(() => setCopied(false), 2000);
		} catch (err) {
			console.error('Failed to copy:', err);
		}
	};

	return (
		<button
			onClick={handleCopy}
			className="mb-8 flex w-full items-center justify-center gap-2 rounded-lg border border-zinc-200 bg-zinc-50 px-4 py-3 text-sm font-mono text-zinc-700 transition-colors hover:border-zinc-300 hover:bg-zinc-100"
		>
			{copied ? (
				<Check className="h-4 w-4 text-emerald-600" />
			) : (
				<Terminal className="h-4 w-4 text-zinc-500" />
			)}
			<span className="truncate">{command}</span>
			{!copied && <Copy className="h-3.5 w-3.5 ml-auto text-zinc-400" />}
		</button>
	);
};

const PricingCard = ({ tier, index, showCloudNotice = false }: { tier: typeof pricingTiers[0]; index: number; showCloudNotice?: boolean }) => {
	const Icon = tier.icon;

	return (
		<motion.div
			initial={{ opacity: 0, y: 20 }}
			animate={{ opacity: 1, y: 0 }}
			transition={{ duration: 0.5, delay: index * 0.1 }}
			className={`relative flex flex-col rounded-2xl border ${
				tier.highlight
					? 'border-zinc-900 bg-zinc-900 text-white'
					: 'border-zinc-200 bg-white text-zinc-900'
			} p-8`}
		>
			<div className="mb-6">
				<div className={`mb-4 flex h-12 w-12 items-center justify-center rounded-xl ${
					tier.highlight ? 'bg-white/10' : 'bg-zinc-100'
				}`}>
					<Icon className={`h-6 w-6 ${tier.highlight ? 'text-white' : 'text-zinc-700'}`} />
				</div>
				<h3 className="text-xl font-semibold">{tier.name}</h3>
				<p className={`mt-1 text-sm ${tier.highlight ? 'text-zinc-400' : 'text-zinc-500'}`}>
					{tier.description}
				</p>
			</div>

			<div className="mb-6">
				<div className="flex items-baseline gap-2">
					<span className="text-3xl font-bold">{tier.price}</span>
				</div>
				<p className={`text-sm ${tier.highlight ? 'text-zinc-400' : 'text-zinc-500'}`}>
					{tier.priceSuffix}
				</p>
			</div>

			{tier.copyCommand ? (
				<CopyButton command={tier.copyCommand} highlight={tier.highlight} />
			) : (
				<a
					href={tier.ctaHref}
					className={`mb-8 flex items-center justify-center gap-2 rounded-lg px-4 py-3 text-sm font-medium transition-colors ${
						tier.highlight
							? 'bg-white text-zinc-900 hover:bg-zinc-100'
							: 'bg-zinc-900 text-white hover:bg-zinc-700'
					}`}
				>
					{tier.cta}
					<ArrowRight className="h-4 w-4" />
				</a>
			)}

			<ul className="space-y-3">
				{tier.features.map((feature) => (
					<li key={feature.text} className="flex items-start gap-3">
						<Check className={`mt-0.5 h-4 w-4 flex-shrink-0 ${
							tier.highlight ? 'text-emerald-400' : 'text-emerald-600'
						}`} />
						<span className={`text-sm ${tier.highlight ? 'text-zinc-300' : 'text-zinc-600'}`}>
							{feature.text}
						</span>
					</li>
				))}
			</ul>

			{showCloudNotice && (
				<div className="mt-6 pt-6 border-t border-zinc-200">
					<a
						href="https://dashboard.rivet.dev"
						className="group flex items-center gap-3 rounded-lg bg-zinc-50 p-4 transition-colors hover:bg-zinc-100"
					>
						<Cloud className="h-5 w-5 text-zinc-500 group-hover:text-zinc-700 transition-colors" />
						<div className="flex-1">
							<p className="text-sm font-medium text-zinc-900">Deploy on Rivet Cloud</p>
							<p className="text-xs text-zinc-500">Scale your agents with managed infrastructure</p>
						</div>
						<ArrowRight className="h-4 w-4 text-zinc-400 group-hover:text-zinc-700 transition-colors" />
					</a>
				</div>
			)}
		</motion.div>
	);
};

const FAQSection = () => {
	const faqs = [
		{
			question: 'Is agentOS really free?',
			answer: 'Yes. agentOS is open source under the Apache 2.0 license. You can run it on your own infrastructure at no cost. Rivet Cloud is a paid service for those who want managed infrastructure.',
		},
		{
			question: 'What is the difference between self-hosted and Rivet Cloud?',
			answer: 'Self-hosted means you run agentOS on your own servers. You handle deployment, scaling, and maintenance. Rivet Cloud is a fully managed service where we handle all of that for you.',
		},
		{
			question: 'Can I switch from self-hosted to Rivet Cloud later?',
			answer: 'Absolutely. agentOS uses the same API whether you self-host or use Rivet Cloud. You can migrate with minimal code changes.',
		},
		{
			question: 'What support is available for open source users?',
			answer: 'Open source users can get help through our Discord community and GitHub issues. Enterprise customers receive dedicated support channels with guaranteed response times.',
		},
		{
			question: 'Do you offer volume discounts?',
			answer: 'Yes. Contact our sales team for custom pricing on high-volume usage or enterprise deployments.',
		},
	];

	return (
		<motion.section
			initial={{ opacity: 0, y: 20 }}
			whileInView={{ opacity: 1, y: 0 }}
			viewport={{ once: true }}
			transition={{ duration: 0.5 }}
			className="border-t border-zinc-200 px-6 py-24"
		>
			<div className="mx-auto max-w-3xl">
				<h2 className="mb-12 text-center text-3xl font-normal tracking-tight text-zinc-900 md:text-4xl">
					Frequently asked questions
				</h2>
				<div className="space-y-6">
					{faqs.map((faq) => (
						<div key={faq.question} className="border-b border-zinc-200 pb-6">
							<h3 className="mb-2 text-lg font-medium text-zinc-900">{faq.question}</h3>
							<p className="text-sm leading-relaxed text-zinc-500">{faq.answer}</p>
						</div>
					))}
				</div>
			</div>
		</motion.section>
	);
};

const CTASection = () => (
	<motion.section
		initial={{ opacity: 0, y: 20 }}
		whileInView={{ opacity: 1, y: 0 }}
		viewport={{ once: true }}
		transition={{ duration: 0.5 }}
		className="border-t border-zinc-200 px-6 py-24"
	>
		<div className="mx-auto max-w-3xl text-center">
			<h2 className="mb-4 text-3xl font-normal tracking-tight text-zinc-900 md:text-4xl">
				Ready to get started?
			</h2>
			<p className="mb-8 text-base leading-relaxed text-zinc-500 md:text-lg">
				Deploy agentOS today. Start with the open source version or try Rivet Cloud for free.
			</p>
			<div className="flex flex-col items-center justify-center gap-4 sm:flex-row">
				<a
					href="/docs/agent-os"
					className="inline-flex items-center gap-2 rounded-lg bg-zinc-900 px-6 py-3 text-sm font-medium text-white transition-colors hover:bg-zinc-700"
				>
					Start for Free
					<ArrowRight className="h-4 w-4" />
				</a>
				<a
					href="/sales"
					className="inline-flex items-center gap-2 rounded-lg border border-zinc-200 bg-white px-6 py-3 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-50"
				>
					<Headphones className="h-4 w-4" />
					Talk to Sales
				</a>
			</div>
		</div>
	</motion.section>
);

export default function AgentOSPricingPage() {
	return (
		<div className="min-h-screen overflow-x-hidden bg-white font-sans text-zinc-600 selection:bg-zinc-200 selection:text-zinc-900">
			<main>
				{/* Hero */}
				<section className="px-6 pt-24 pb-16 md:pt-32">
					<div className="mx-auto max-w-5xl text-center">
						<motion.div
							initial={{ opacity: 0, y: 20 }}
							animate={{ opacity: 1, y: 0 }}
							transition={{ duration: 0.5 }}
						>
							<h1 className="mb-4 text-4xl font-normal tracking-tight text-zinc-900 md:text-5xl lg:text-6xl">
								Free and open source.
							</h1>
							<p className="mx-auto max-w-2xl text-base leading-relaxed text-zinc-500 md:text-lg">
								agentOS is Apache 2.0 licensed and free to self-host. Use Rivet Cloud for managed infrastructure, or contact us for enterprise support.
							</p>
						</motion.div>
					</div>
				</section>

				{/* Pricing Cards */}
				<section className="px-6 pb-24">
					<div className="mx-auto max-w-4xl">
						<div className="grid gap-8 md:grid-cols-2">
							{pricingTiers.map((tier, index) => (
								<PricingCard key={tier.name} tier={tier} index={index} showCloudNotice={index === 0} />
							))}
						</div>
					</div>
				</section>

				<FAQSection />
				<CTASection />
			</main>
		</div>
	);
}
