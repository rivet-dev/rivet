'use client';

import { motion } from 'framer-motion';
import { SECTION_H2_CLASS } from '../typography';
import { AnimatedAgentOSLogo } from '../AnimatedAgentOSLogo';
import { benchColdStart, benchWorkloads } from '@/data/bench-agentos';

// Multipliers derive from the shared benchmark data in bench-agentos.ts. The
// selection follows the agent-os hero stat convention (shell workload, p50,
// AWS ARM) so the two sites cite identical numbers once both consume the same
// data. Each stat links to the benchmark it claims.
const stats = [
	{
		value: Math.round(benchColdStart[0].sandbox / benchColdStart[0].agentOS),
		label: 'faster cold starts',
		href: 'https://agentos-sdk.dev/#bench-cold-start',
	},
	{
		value: benchWorkloads.shell.memory.multiplier.split('x')[0],
		label: 'less memory',
		href: 'https://agentos-sdk.dev/#bench-memory',
	},
	{
		value:
			benchWorkloads.shell.cost.find((tier) => tier.label === 'AWS ARM')?.ratio ??
			Math.min(...benchWorkloads.shell.cost.map((tier) => tier.ratio)),
		label: 'cheaper to run',
		href: 'https://agentos-sdk.dev/#bench-cost',
	},
];

export const AgentOSSection = () => (
	<section className='relative border-t border-ink/10 px-6 py-16 lg:py-24'>
		<div className='mx-auto w-full max-w-7xl'>
			<div className='mb-12 max-w-3xl'>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
				>
					<h2 className={SECTION_H2_CLASS}>Give your agent an OS without a sandbox.</h2>
				</motion.div>
			</div>

			<motion.div
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5, delay: 0.1 }}
				className='border border-ink/10 bg-white/55 p-6 md:p-10'
			>
				<AnimatedAgentOSLogo className='h-10 w-auto md:h-11' drawDurationSec={2} />
				<p className='mt-6 max-w-2xl text-sm leading-relaxed text-ink-soft md:text-base'>
					<span className='block'>A lightweight library that gives agents a file system, networking, bash, Python, and Node.</span>
					<span className='mt-2 block'>No containers, no VMs. Powered by Rivet Actors.</span>
					<span className='mt-2 block'>Isolated with Wasm. Runs in your own backend.</span>
				</p>

				<p className='mt-8 text-sm font-medium text-accent'>Compared to running agents in a sandbox microVM:</p>
				<div className='mt-3 grid border-y border-ink/10 sm:grid-cols-3 sm:divide-x sm:divide-ink/10'>
					{stats.map((stat) => (
						<a
							key={stat.href}
							href={stat.href}
							target='_blank'
							rel='noopener noreferrer'
							className='group flex flex-col gap-1 border-t border-ink/10 py-5 first:border-t-0 sm:border-t-0 sm:px-8 sm:first:pl-0 sm:last:pr-0'
						>
							<span className='text-3xl font-medium tabular-nums text-ink transition-colors group-hover:text-pine'>
								{stat.value}×
							</span>
							<span className='text-sm font-medium text-ink transition-colors group-hover:text-pine'>{stat.label}</span>
						</a>
					))}
				</div>

				<a
					href='https://agentos-sdk.dev'
					target='_blank'
					rel='noopener noreferrer'
					className='group mt-8 inline-flex items-center gap-1.5 text-sm font-medium text-pine'
				>
					Explore agentOS
					<span aria-hidden='true' className='transition-transform duration-200 group-hover:translate-x-0.5'>
						→
					</span>
				</a>
			</motion.div>
		</div>
	</section>
);
