'use client';

import { useState, useRef, useEffect } from 'react';
import { ArrowDown, ArrowUp, Info } from 'lucide-react';
import { motion } from 'framer-motion';

interface BarEntry {
	label: string;
	value: string;
	/** Numeric value used to compute bar width proportionally. */
	rawValue: number;
	highlight?: boolean;
	infinite?: boolean;
	/** Hover note explaining the benchmark context. */
	note?: string;
}

interface BenchmarkCard {
	title: string;
	subtitle: string;
	direction: 'lower is better' | 'higher is better';
	bars: BarEntry[];
	/** Hide bar charts and show only labels and values. */
	noBars?: boolean;
	/** Use a logarithmic scale for bar widths. */
	logarithmic?: boolean;
}

function Tooltip({ note }: { note: string }) {
	const [visible, setVisible] = useState(false);
	const [position, setPosition] = useState<'above' | 'below'>('above');
	const tooltipRef = useRef<HTMLDivElement>(null);
	const iconRef = useRef<HTMLButtonElement>(null);

	useEffect(() => {
		if (visible && iconRef.current) {
			const rect = iconRef.current.getBoundingClientRect();
			// Show below if too close to the top of the viewport
			setPosition(rect.top < 80 ? 'below' : 'above');
		}
	}, [visible]);

	return (
		<span className='relative inline-flex'>
			<button
				ref={iconRef}
				type='button'
				className='ml-1 inline-flex items-center text-zinc-600 transition-colors hover:text-zinc-400'
				onMouseEnter={() => setVisible(true)}
				onMouseLeave={() => setVisible(false)}
				onClick={() => setVisible((v) => !v)}
				aria-label='More info'
			>
				<Info className='h-3 w-3' />
			</button>
			{visible && (
				<div
					ref={tooltipRef}
					className={`absolute left-1/2 z-50 w-52 -translate-x-1/2 rounded-lg border border-white/10 bg-zinc-900 px-3 py-2 text-[11px] leading-relaxed text-zinc-300 shadow-xl ${
						position === 'above' ? 'bottom-full mb-2' : 'top-full mt-2'
					}`}
				>
					{note}
				</div>
			)}
		</span>
	);
}

/** Compute bar widths as percentages from raw values, with a minimum of 2%. */
function computeBarWidths(bars: BarEntry[], logarithmic?: boolean): number[] {
	const finiteValues = bars.filter((b) => !b.infinite).map((b) => b.rawValue);
	const maxValue = Math.max(...finiteValues);
	if (logarithmic) {
		const logMax = Math.log10(Math.max(maxValue, 1));
		return bars.map((bar) => {
			if (bar.infinite) return 100;
			if (bar.rawValue <= 0) return 2;
			return Math.max((Math.log10(bar.rawValue) / logMax) * 100, 2);
		});
	}
	return bars.map((bar) => {
		if (bar.infinite) return 100;
		if (maxValue === 0) return 2;
		return Math.max((bar.rawValue / maxValue) * 100, 2);
	});
}

const benchmarks: BenchmarkCard[] = [
	{
		title: 'Cold Start',
		subtitle: 'Time to first request',
		direction: 'lower is better',
		bars: [
			{
			label: 'Rivet Actor',
			value: '~20ms',
			rawValue: 20,
			highlight: true,
			note: 'Includes durable state init, not just a process spawn. No actor key, so no cross-region locking. Measured with Node.js and FoundationDB.',
		},
			{
				label: 'Kubernetes Pod',
				value: '~6s',
				rawValue: 6000,
				note: 'Node.js 24 Alpine image (56MB compressed) on AWS EKS with a pre-provisioned m5.large node. Breakdown: ~1s image pull and extraction, ~3-4s scheduling and container runtime setup, ~1s container start.',
			},
			{
				label: 'Virtual Machine',
				value: '~30s',
				rawValue: 30000,
				note: 'AWS EC2 t3.nano instance from launch to SSH-ready, using an Amazon Linux 2 AMI. t3.nano is the smallest available EC2 instance (512MB RAM).',
			},
		],
	},
	{
		title: 'Memory Per Instance',
		subtitle: 'Overhead per instance',
		direction: 'lower is better',
		bars: [
			{
				label: 'Rivet Actor',
				value: '~0.6KB',
				rawValue: 0.6,
				highlight: true,
				note: 'RSS (resident set size) delta divided by actor count, measured by spawning 10,000 actors in Node.js v24 on Linux x86.',
			},
			{
				label: 'Kubernetes Pod',
				value: '~50MB',
				rawValue: 50,
				note: 'Minimum idle Node.js container on Linux x86: Node.js v24 runtime (~43MB RSS), containerd-shim (~3MB), pause container (~1MB), and kubelet per-pod tracking (~2MB).',
			},
			{
				label: 'Virtual Machine',
				value: '~512MB',
				rawValue: 512,
				note: 'AWS EC2 t3.nano, the smallest available EC2 instance with 512MB allocated memory.',
			},
		],
	},
	{
		title: 'Read Latency',
		subtitle: 'State read latency',
		direction: 'lower is better',
		bars: [
			{
				label: 'Rivet Actor',
				value: '0ms',
				rawValue: 0.01,
				highlight: true,
				note: 'State is read from co-located SQLite/KV storage on the same machine as the actor, with no network round-trip.',
			},
			{
				label: 'Redis',
				value: '~1ms',
				rawValue: 1,
				note: 'AWS ElastiCache Redis (cache.t3.micro) in the same availability zone as the application.',
			},
			{
				label: 'Postgres',
				value: '~5ms',
				rawValue: 5,
				note: 'AWS RDS PostgreSQL (db.t3.micro) in the same availability zone as the application.',
			},
		],
	},
	{
		title: 'Idle Cost',
		subtitle: 'Cost when not in use',
		direction: 'lower is better',
		bars: [
			{
				label: 'Rivet Actor',
				value: '$0',
				rawValue: 0.01,
				highlight: true,
				note: 'Assumes Rivet Actors running on a serverless platform. Actors scale to zero with no idle infrastructure costs. Traditional container deployments may incur idle costs.',
			},
			{
				label: 'Virtual Machine',
				value: '~$5/mo',
				rawValue: 5,
				note: 'AWS EC2 t3.nano ($0.0052/hr compute + $1.60/mo for 20GB gp3 storage) running 24/7. t3.nano is the smallest available EC2 instance (512MB RAM).',
			},
			{
				label: 'Kubernetes Cluster',
				value: '~$85/mo',
				rawValue: 85,
				note: 'AWS EKS control plane ($73/mo) plus a single t3.nano worker node with 20GB gp3 storage, running 24/7. t3.nano is the smallest available EC2 instance (512MB RAM).',
			},
		],
	},
	{
		title: 'Horizontal Scale',
		subtitle: 'Maximum capacity',
		direction: 'higher is better',
		noBars: true,
		bars: [
			{
				label: 'Rivet Actors',
				value: 'Infinite',
				rawValue: 0,
				highlight: true,
				note: 'Rivet Actors scale linearly by adding nodes with no single cluster size limit.',
			},
			{
				label: 'Kubernetes',
				value: '~5k nodes',
				rawValue: 5000,
				note: 'Kubernetes officially supports clusters of up to 5,000 nodes per the Kubernetes scalability documentation.',
			},
			{ label: 'Postgres', value: '1 primary', rawValue: 1 },
		],
	},
	{
		title: 'Multi-Region',
		subtitle: 'Deploy actors close to your users',
		direction: 'lower is better',
		noBars: true,
		bars: [
			{
				label: 'Rivet',
				value: 'Global edge network',
				rawValue: 0,
				highlight: true,
				note: 'Rivet automatically spawns actors near your users and handles routing across regions for a seamless edge network.',
			},
			{
				label: 'Traditional Deployment',
				value: '1 region',
				rawValue: 0,
			},
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
						How Actors Compare
					</h2>
					<p className='text-base leading-relaxed text-zinc-500'>
						Rivet Actors vs. traditional infrastructure.
					</p>
				</motion.div>

				<div className='grid grid-cols-1 gap-6 md:grid-cols-2'>
					{benchmarks.map((card, idx) => {
						const widths = computeBarWidths(card.bars, card.logarithmic);
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
								{card.bars.map((bar, barIdx) => (
									<div key={bar.label} className='flex flex-col gap-1'>
										<div className='flex items-center justify-between text-xs'>
											<span className={`inline-flex items-center ${bar.highlight ? 'text-white' : 'text-zinc-500'}`}>
												{bar.label}
												{bar.note && <Tooltip note={bar.note} />}
											</span>
											<span className={bar.highlight ? 'font-medium text-[#FF4500]' : 'text-zinc-500'}>
												{bar.value}
											</span>
										</div>
										{!card.noBars && (
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
														whileInView={{ width: `${widths[barIdx]}%` }}
														viewport={{ once: true }}
														transition={{ duration: 0.6, delay: 0.2 }}
													/>
												</div>
											)}
										</div>
										)}
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
