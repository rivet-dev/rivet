'use client';

import { ArrowDown, ArrowUp } from 'lucide-react';
import { motion } from 'framer-motion';

interface BarEntry {
	label: string;
	value: string;
	/** Width as percentage (0-100). */
	width: number;
	highlight?: boolean;
	infinite?: boolean;
}

interface BenchmarkCard {
	title: string;
	subtitle: string;
	direction: 'lower is better' | 'higher is better';
	bars: BarEntry[];
}


const benchmarks: BenchmarkCard[] = [
	{
		title: 'Cold Start',
		subtitle: 'Time to first request',
		direction: 'lower is better',
		bars: [
			{ label: 'Rivet Actor', value: '~5ms', width: 1, highlight: true },
			{ label: 'Firecracker microVM', value: '~125ms', width: 3 },
			{ label: 'Kubernetes Pod', value: '~5s', width: 17 },
			{ label: 'EC2 Instance', value: '~30s', width: 100 },
		],
	},
	{
		title: 'Memory Per Instance',
		subtitle: 'Overhead per instance',
		direction: 'lower is better',
		bars: [
			{ label: 'Rivet Actor', value: '~2MB', width: 2, highlight: true },
			{ label: 'Firecracker microVM', value: '~128MB', width: 25 },
			{ label: 'EC2 Instance', value: '~512MB+', width: 100 },
		],
	},
	{
		title: 'Read Latency',
		subtitle: 'State access time',
		direction: 'lower is better',
		bars: [
			{ label: 'Rivet Actor', value: '0ms', width: 1, highlight: true },
			{ label: 'Redis', value: '~1ms', width: 20 },
			{ label: 'Postgres', value: '~5ms', width: 100 },
		],
	},
	{
		title: 'Horizontal Scale',
		subtitle: 'Maximum capacity',
		direction: 'higher is better',
		bars: [
			{ label: 'Rivet Actors', value: '∞', width: 100, highlight: true, infinite: true },
			{ label: 'Kubernetes', value: '~5k nodes', width: 40 },
			{ label: 'Postgres', value: '1 primary', width: 8 },
		],
	},
	{
		title: 'Edge Latency',
		subtitle: 'Latency to your users on the edge',
		direction: 'lower is better',
		bars: [
			{ label: 'Rivet Cloud Edge Network', value: '~50ms', width: 25, highlight: true },
			{ label: 'Single Region', value: '~200ms+', width: 100 },
		],
	},
	{
		title: 'Idle Cost',
		subtitle: 'Cost when not in use',
		direction: 'lower is better',
		bars: [
			{ label: 'Rivet Actor', value: '$0', width: 1, highlight: true },
			{ label: 'Kubernetes Cluster', value: '~$70/mo', width: 70 },
			{ label: 'EC2 Instance', value: '~$100/mo', width: 100 },
		],
	},
];

export const BenchmarksSection = () => {
	return (
		<section className='border-t border-white/10 px-6 py-16 lg:py-24'>
			<div className='mx-auto w-full max-w-7xl'>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className='mb-10'
				>
					<h2 className='mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl'>
						Benchmarks
					</h2>
					<p className='text-base leading-relaxed text-zinc-500'>
						How Rivet Actors compare to traditional infrastructure.
					</p>
				</motion.div>

				<div className='grid grid-cols-1 gap-6 md:grid-cols-2'>
					{benchmarks.map((card, idx) => {
						const bars = card.bars;
						return (
						<motion.div
							key={card.title}
							initial={{ opacity: 0, y: 16 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.4, delay: idx * 0.05 }}
							className='rounded-xl border border-white/10 bg-white/[0.03] p-6'
						>
							<div className='mb-4 flex items-start justify-between'>
								<div>
									<h3 className='text-sm font-medium text-white'>{card.title}</h3>
									<p className='text-xs text-zinc-500'>{card.subtitle}</p>
								</div>
								<span className='flex items-center gap-1 text-[10px] uppercase tracking-wider text-zinc-600'>
									{card.direction === 'lower is better' ? (
										<ArrowDown className='h-3 w-3' />
									) : (
										<ArrowUp className='h-3 w-3' />
									)}
									{card.direction}
								</span>
							</div>

							<div className='flex flex-col gap-3'>
								{bars.map((bar) => (
									<div key={bar.label} className='flex flex-col gap-1'>
										<div className='flex items-center justify-between text-xs'>
											<span className={bar.highlight ? 'text-white' : 'text-zinc-500'}>
												{bar.label}
											</span>
											<span className={bar.highlight ? 'font-medium text-[#FF4500]' : 'text-zinc-500'}>
												{bar.value}
											</span>
										</div>
										<div className={`w-full ${bar.infinite ? 'relative' : ''}`}>
											{bar.infinite ? (
												<>
													<div className='h-1.5 w-full overflow-hidden rounded-full bg-white/5'>
														<div className='h-full w-full rounded-full bg-[#FF4500]' />
													</div>
													<div
														className='absolute right-8 top-1/2 -translate-y-1/2 rotate-12'
														style={{
															width: '6px',
															height: '14px',
															backgroundColor: '#020202',
															borderLeft: '1.5px solid #FF4500',
															borderRight: '1.5px solid #FF4500',
														}}
													/>
												</>
											) : (
												<div className='h-1.5 w-full overflow-hidden rounded-full bg-white/5'>
													<motion.div
														className={`h-full rounded-full ${bar.highlight ? 'bg-[#FF4500]' : 'bg-zinc-600'}`}
														initial={{ width: 0 }}
														whileInView={{ width: `${Math.max(bar.width, 2)}%` }}
														viewport={{ once: true }}
														transition={{ duration: 0.6, delay: 0.2 }}
													/>
												</div>
											)}
										</div>
									</div>
								))}
							</div>
						</motion.div>
					);
					})}
				</div>
			</div>
		</section>
	);
};
