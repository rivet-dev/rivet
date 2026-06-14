'use client';

import { motion } from 'framer-motion';
import { ArrowRight, Check } from 'lucide-react';
import { SECTION_H2_CLASS, SUBTITLE_CLASS } from '../typography';
import { PerimeterDiagram } from '../editorial/PerimeterDiagram';

const points = [
	'Air-gapped and on-prem: no outbound calls, no telemetry leaving your boundary',
	'Embed in your customers’ VPCs: they keep their data, you keep your product',
	'FedRAMP, HIPAA, sovereign clouds: stay inside the boundary your controls already cover',
];

export const OnPremSection = () => (
	<section className='border-t border-ink/10 px-6 py-16 md:py-32'>
		<div className='mx-auto max-w-7xl'>
			<div className='grid grid-cols-1 items-center gap-12 lg:grid-cols-2 lg:gap-20'>
				<div>
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.05 }}
					>
												<h2 className={`mb-4 ${SECTION_H2_CLASS}`}>Run it where your data lives.</h2>
					</motion.div>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className='text-base leading-relaxed text-ink-soft md:text-lg'
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
							<li key={point} className='flex items-start gap-3 text-sm leading-relaxed text-ink-soft'>
								<span className='mt-0.5 flex h-5 w-5 flex-none items-center justify-center rounded-full border border-pine/30'>
									<Check className='h-3 w-3 text-pine' />
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
							className='inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-ink px-4 py-2 text-sm font-medium text-cream transition-colors hover:bg-ink/85'
						>
							Talk to an engineer
							<ArrowRight className='h-4 w-4' />
						</a>
						<a
							href='/enterprise'
							className='inline-flex items-center justify-center whitespace-nowrap rounded-md border border-ink/20 px-4 py-2 text-sm text-ink-soft transition-colors hover:border-ink/40 hover:text-ink'
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
