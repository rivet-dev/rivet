'use client';

import { Database, Globe, Infinity, Layers, Wifi, GitBranch, ArrowRight } from 'lucide-react';
import { motion } from 'framer-motion';
import { SECTION_H2_CLASS, SUBTITLE_CLASS } from '../typography';

const actorFeatures = [
	{
		icon: Database,
		title: 'In-memory state',
		description: 'Co-located with compute for instant reads and writes.',
	},
	{
		icon: Infinity,
		title: 'Runs indefinitely, sleeps when idle',
		description: 'Long-lived when active, hibernates when idle.',
	},
	{
		icon: Layers,
		title: 'Scales to zero, bursts to thousands',
		description: 'Sleeps at near-zero cost when idle, fans out when traffic spikes.',
	},
	{
		icon: Globe,
		title: 'Global edge network',
		description: 'Deploy close to your users without complexity.',
	},
	{
		icon: Wifi,
		title: 'WebSockets',
		description: 'Real-time bidirectional streaming built in.',
	},
	{
		icon: GitBranch,
		title: 'Workflows, Queues, Scheduling',
		description: 'Multi-step operations, durable queues, and timers.',
	},
];

export const ActorsPrimitiveSection = () => (
	<section className='relative border-t border-ink/10 px-6 py-16 lg:py-24'>
		<div className='mx-auto w-full max-w-7xl'>
			<div className='max-w-3xl'>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
				>
					<h2 className={SECTION_H2_CLASS}>One actor per agent, per session, per user.</h2>
				</motion.div>
				<motion.p
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.05 }}
					className={SUBTITLE_CLASS}
				>
					Rivet Actors give agents and realtime apps durable, stateful compute in your existing Node.js or Bun backend.
				</motion.p>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.1 }}
					className='mt-6 flex flex-wrap gap-3'
				>
					<a
						href='/docs'
						className='inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-ink px-4 py-2 text-sm font-medium text-cream transition-colors hover:bg-ink/85'
					>
						Documentation
					</a>
					<a
						href='/actors'
						className='inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-ink/20 px-4 py-2 text-sm text-ink-soft transition-colors hover:border-ink/40 hover:text-ink'
					>
						Details
						<ArrowRight className='h-3.5 w-3.5' />
					</a>
				</motion.div>
			</div>

			<div className='mt-12 grid sm:grid-cols-2 sm:gap-x-8 lg:grid-cols-3'>
				{actorFeatures.map((feature, idx) => {
					const Icon = feature.icon;
					return (
						<motion.div
							key={feature.title}
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: idx * 0.05 }}
							className='border-t border-ink/10 py-6'
						>
							<Icon className='mb-2 h-4 w-4 text-olive' />
							<h3 className='mb-1 text-sm font-medium text-ink'>{feature.title}</h3>
							<p className='text-sm leading-relaxed text-ink-soft'>{feature.description}</p>
						</motion.div>
					);
				})}
			</div>
		</div>
	</section>
);
