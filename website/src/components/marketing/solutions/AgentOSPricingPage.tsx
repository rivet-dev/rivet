'use client';

import { useState } from 'react';
import { motion } from 'framer-motion';
import {
	ArrowRight,
	Check,
	Server,
	Cloud,
	Headphones,
	Copy,
} from 'lucide-react';
import { FaqSection } from '@/components/faq/FaqSection';
import { agentOsPricingFaqs } from '@/data/faqs/agent-os-pricing';
import { HERO_H1_CLASS, SECTION_H2_CLASS } from '@/components/marketing/typography';

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
		priceSuffix: '',
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

// Ink command strip with a copy affordance. The install command is the free
// tier's call to action, so it gets the page's single code moment treatment.
const CopyButton = ({ command }: { command: string }) => {
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
			className='selection-paper mb-8 flex w-full items-center gap-2.5 rounded-lg border border-ink/20 bg-ink px-4 py-3 font-mono text-[13px] text-cream/85 transition-colors hover:border-ink/40'
		>
			<span aria-hidden='true' className='select-none text-cream/70'>
				$
			</span>
			<span className='truncate'>{command}</span>
			{copied ? (
				<Check className='ml-auto h-3.5 w-3.5 text-cream/70' />
			) : (
				<Copy className='ml-auto h-3.5 w-3.5 text-cream/45' />
			)}
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
			className='relative flex flex-col border border-ink/10 bg-white/55 p-8'
		>
			<div className='mb-6'>
				<div className='mb-4 flex h-12 w-12 items-center justify-center rounded-xl border border-ink/10'>
					<Icon className='h-6 w-6 text-ink-soft' />
				</div>
				<h3 className='text-xl font-medium text-ink'>{tier.name}</h3>
				<p className='mt-1 text-sm text-ink-soft'>{tier.description}</p>
			</div>

			<div className='mb-6'>
				<div className='flex items-baseline gap-2'>
					<span className='text-3xl font-medium text-ink'>{tier.price}</span>
				</div>
				{tier.priceSuffix && (
					<p className='font-mono text-xs uppercase tracking-[0.16em] text-ink-faint'>{tier.priceSuffix}</p>
				)}
			</div>

			{tier.copyCommand ? (
				<CopyButton command={tier.copyCommand} />
			) : (
				<a
					href={tier.ctaHref}
					className='mb-8 flex items-center justify-center gap-2 rounded-lg bg-ink px-4 py-3 text-sm font-medium text-cream transition-colors hover:bg-ink/85'
				>
					{tier.cta}
					<ArrowRight className='h-4 w-4' />
				</a>
			)}

			<ul className='space-y-3'>
				{tier.features.map((feature) => (
					<li key={feature.text} className='flex items-start gap-3'>
						<Check className='mt-0.5 h-4 w-4 flex-shrink-0 text-ink' />
						<span className='text-sm text-ink-soft'>{feature.text}</span>
					</li>
				))}
			</ul>

			{showCloudNotice && (
				<div className='mt-6 border-t border-ink/10 pt-6'>
					<a
						href='https://dashboard.rivet.dev'
						className='group flex items-center gap-3 rounded-lg border border-ink/10 p-4 transition-colors hover:border-ink/25'
					>
						<Cloud className='h-5 w-5 text-ink-soft' />
						<div className='flex-1'>
							<p className='text-sm font-medium text-ink'>Deploy on Rivet Cloud</p>
							<p className='text-xs text-ink-soft'>Scale your agents with managed infrastructure</p>
						</div>
						<ArrowRight className='h-4 w-4 text-ink-faint transition-colors group-hover:text-ink' />
					</a>
				</div>
			)}
		</motion.div>
	);
};

const CTASection = () => (
	<motion.section
		initial={{ opacity: 0, y: 20 }}
		whileInView={{ opacity: 1, y: 0 }}
		viewport={{ once: true }}
		transition={{ duration: 0.5 }}
		className='border-t border-ink/10 px-6 py-24'
	>
		<div className='mx-auto max-w-3xl text-center'>
			<h2 className={`mb-4 ${SECTION_H2_CLASS}`}>Ready to get started?</h2>
			<p className='mb-8 text-base leading-relaxed text-ink-soft md:text-lg'>
				Deploy agentOS today. Start with the open source version or try Rivet Cloud for free.
			</p>
			<div className='flex flex-col items-center justify-center gap-4 sm:flex-row'>
				<a
					href='/docs/agent-os'
					className='inline-flex items-center gap-2 rounded-md bg-accent-deep px-6 py-3 text-sm font-medium text-white transition-colors hover:bg-accent'
				>
					Start for Free
					<ArrowRight className='h-4 w-4' />
				</a>
				<a
					href='/sales'
					className='inline-flex items-center gap-2 rounded-md border border-ink/20 px-6 py-3 text-sm font-medium text-ink-soft transition-colors hover:border-ink/40 hover:text-ink'
				>
					<Headphones className='h-4 w-4' />
					Talk to Sales
				</a>
			</div>
		</div>
	</motion.section>
);

export default function AgentOSPricingPage() {
	return (
		<div className='paper-grain min-h-screen overflow-x-hidden font-sans text-ink-soft'>
			<main>
				{/* Hero */}
				<section className='px-6 pt-24 pb-16 md:pt-32'>
					<div className='mx-auto max-w-5xl text-center'>
						<motion.div
							initial={{ opacity: 0, y: 20 }}
							animate={{ opacity: 1, y: 0 }}
							transition={{ duration: 0.5 }}
						>
							<h1 className={`mb-4 ${HERO_H1_CLASS}`}>Free and open source.</h1>
							<p className='mx-auto max-w-2xl text-base leading-relaxed text-ink-soft md:text-lg'>
								agentOS is Apache 2.0 licensed and free to self-host. Use Rivet Cloud for managed infrastructure, or contact us for enterprise support.
							</p>
						</motion.div>
					</div>
				</section>

				{/* Pricing Cards */}
				<section className='px-6 pb-16 md:pb-32'>
					<div className='mx-auto max-w-4xl'>
						<div className='grid gap-6 md:grid-cols-2'>
							{pricingTiers.map((tier, index) => (
								<PricingCard key={tier.name} tier={tier} index={index} showCloudNotice={index === 0} />
							))}
						</div>
					</div>
				</section>

				{/* No motion wrapper here. The FAQ must stay visible without JavaScript
				    because the page's FaqJsonLd requires the content to be on-page. */}
				<FaqSection items={agentOsPricingFaqs} theme='light' />
				<CTASection />
			</main>
		</div>
	);
}
