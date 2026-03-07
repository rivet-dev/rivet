'use client';

import { motion } from 'framer-motion';

const benchmarks = [
	{
		value: '<1ms',
		label: 'cold start time',
	},
	{
		value: '~2MB',
		label: 'memory per actor',
	},
	{
		value: '0ms',
		label: 'added latency',
	},
];

export const BenchmarksSection = () => {
	return (
		<section className='border-t border-white/10 px-6 py-20 lg:py-32'>
			<div className='mx-auto max-w-7xl'>
				<div className='grid grid-cols-1 gap-6 sm:grid-cols-3'>
					{benchmarks.map((stat, idx) => (
						<motion.div
							key={stat.label}
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: idx * 0.05 }}
							className='rounded-xl border border-white/10 bg-white/[0.03] px-8 py-10 text-center'
						>
							<div className='mb-2 text-5xl font-light tracking-tight text-white md:text-6xl'>
								{stat.value}
							</div>
							<div className='text-sm text-zinc-500'>{stat.label}</div>
						</motion.div>
					))}
				</div>
			</div>
		</section>
	);
};
