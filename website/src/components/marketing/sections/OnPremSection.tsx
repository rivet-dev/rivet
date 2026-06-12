'use client';

import { motion } from 'framer-motion';
import { ArrowRight, Ban, Check } from 'lucide-react';

const points = [
	'Air-gapped and on-prem: no outbound calls, no telemetry leaving your boundary',
	'Embed in your customers’ VPCs: they keep their data, you keep your product',
	'FedRAMP, HIPAA, sovereign clouds: stay inside the boundary your controls already cover',
];

const diagramNodes = [
	{ title: 'Your backend', detail: 'Actors run in your Node.js or Bun process' },
	{ title: 'Rivet Engine', detail: 'Single binary for orchestration and routing' },
	{ title: 'Your storage', detail: 'File system, Postgres, or FoundationDB' },
];

const PerimeterDiagram = () => (
	<div className='relative rounded-xl border border-dashed border-white/20 p-6 md:p-8'>
		<span className='absolute -top-2.5 left-6 bg-black px-2 font-mono text-[11px] uppercase tracking-wider text-zinc-500'>
			Your perimeter
		</span>
		<div className='flex flex-col items-stretch'>
			{diagramNodes.map((node, idx) => (
				<div key={node.title} className='flex flex-col'>
					{idx > 0 && <div className='mx-auto h-5 w-px bg-white/15' />}
					<div className='rounded-lg border border-white/10 bg-white/[0.03] px-4 py-3'>
						<div className='text-sm font-medium text-white'>{node.title}</div>
						<div className='mt-0.5 text-xs text-zinc-500'>{node.detail}</div>
					</div>
				</div>
			))}
		</div>
		<div className='mt-6 flex items-center gap-2 border-t border-white/10 pt-4 font-mono text-[11px] text-zinc-500'>
			<Ban className='h-3.5 w-3.5 text-zinc-600' />
			No outbound connections. No telemetry.
		</div>
	</div>
);

export const OnPremSection = () => (
	<section className='border-t border-white/10 px-6 py-16 md:py-48'>
		<div className='mx-auto max-w-7xl'>
			<div className='grid grid-cols-1 items-center gap-12 lg:grid-cols-2 lg:gap-20'>
				<div>
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.05 }}
						className='mb-4 text-3xl font-medium tracking-[-0.015em] text-white md:text-4xl'
					>
						Run it where your data lives.
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className='text-base leading-relaxed text-zinc-500 md:text-lg'
					>
						A single binary you control. Deploy Rivet inside your VPC, your customer&rsquo;s VPC, or fully air-gapped, with the compliance you already have.
					</motion.p>
					<motion.ul
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.15 }}
						className='mt-8 space-y-4'
					>
						{points.map((point) => (
							<li key={point} className='flex items-start gap-3 text-sm leading-relaxed text-zinc-300'>
								<span className='mt-0.5 flex h-5 w-5 flex-none items-center justify-center rounded-full border border-white/10 bg-white/5'>
									<Check className='h-3 w-3 text-zinc-400' />
								</span>
								{point}
							</li>
						))}
					</motion.ul>
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className='mt-10 flex flex-col gap-3 sm:flex-row'
					>
						<a
							href='/talk-to-an-engineer'
							className='inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200'
						>
							Talk to an engineer
							<ArrowRight className='h-4 w-4' />
						</a>
						<a
							href='/enterprise'
							className='inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white'
						>
							Rivet for Enterprise
						</a>
					</motion.div>
				</div>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.15 }}
				>
					<PerimeterDiagram />
				</motion.div>
			</div>
		</div>
	</section>
);
