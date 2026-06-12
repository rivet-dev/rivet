'use client';

import { useState, useRef, useEffect } from 'react';
import { ArrowDown, ArrowUp, Info } from 'lucide-react';
import { motion } from 'framer-motion';
import { SECTION_H2_CLASS, SUBTITLE_CLASS } from '../typography';
import { Eyebrow } from '../editorial/Eyebrow';
import { InkPanel } from '../editorial/InkPanel';
import { PixelGridChart } from '../art/PixelGridChart';

interface BarEntry {
	label: string;
	value: string;
	highlight?: boolean;
	/** Hover note explaining the benchmark context. */
	note?: string;
}

interface BenchmarkCard {
	title: string;
	subtitle: string;
	direction: 'lower is better' | 'higher is better';
	bars: BarEntry[];
	/** Relative magnitudes (0..1) per entry, rendered as pixel-grid columns. */
	chart: number[];
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
				className='ml-1 inline-flex items-center text-cream/35 transition-colors hover:text-cream/70'
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
					className={`absolute left-1/2 z-50 w-52 -translate-x-1/2 rounded-lg border border-cream/15 bg-ink px-3 py-2 text-[11px] leading-relaxed text-cream/80 shadow-xl ${
						position === 'above' ? 'bottom-full mb-2' : 'top-full mt-2'
					}`}
				>
					{note}
				</div>
			)}
		</span>
	);
}

const benchmarks: BenchmarkCard[] = [
	{
		title: 'Cold Start',
		subtitle: 'Time to first request',
		direction: 'lower is better',
		chart: [0.09, 0.62, 1],
		bars: [
			{
				label: 'Rivet Actor',
				value: '~20ms',
				highlight: true,
				note: 'Includes durable state init, not just a process spawn. No actor key, so no cross-region locking. Measured with Node.js and FoundationDB.',
			},
			{
				label: 'Kubernetes Pod',
				value: '~6s',
				note: 'Node.js 24 Alpine image (56MB compressed) on AWS EKS with a pre-provisioned m5.large node. Breakdown: ~1s image pull and extraction, ~3-4s scheduling and container runtime setup, ~1s container start.',
			},
			{
				label: 'Virtual Machine',
				value: '~30s',
				note: 'AWS EC2 t3.nano instance from launch to SSH-ready, using an Amazon Linux 2 AMI. t3.nano is the smallest available EC2 instance (512MB RAM).',
			},
		],
	},
	{
		title: 'Memory Per Instance',
		subtitle: 'Overhead per instance',
		direction: 'lower is better',
		chart: [0.09, 0.55, 1],
		bars: [
			{
				label: 'Rivet Actor',
				value: '~0.6KB',
				highlight: true,
				note: 'RSS (resident set size) delta divided by actor count, measured by spawning 10,000 actors in Node.js v24 on Linux x86.',
			},
			{
				label: 'Kubernetes Pod',
				value: '~50MB',
				note: 'Minimum idle Node.js container on Linux x86: Node.js v24 runtime (~43MB RSS), containerd-shim (~3MB), pause container (~1MB), and kubelet per-pod tracking (~2MB).',
			},
			{
				label: 'Virtual Machine',
				value: '~512MB',
				note: 'AWS EC2 t3.nano, the smallest available EC2 instance with 512MB allocated memory.',
			},
		],
	},
	{
		title: 'Read Latency',
		subtitle: 'State read latency',
		direction: 'lower is better',
		chart: [0.09, 0.35, 1],
		bars: [
			{
				label: 'Rivet Actor',
				value: '0ms',
				highlight: true,
				note: 'State is read from co-located SQLite/KV storage on the same machine as the actor, with no network round-trip.',
			},
			{
				label: 'Redis',
				value: '~1ms',
				note: 'AWS ElastiCache Redis (cache.t3.micro) in the same availability zone as the application.',
			},
			{
				label: 'Postgres',
				value: '~5ms',
				note: 'AWS RDS PostgreSQL (db.t3.micro) in the same availability zone as the application.',
			},
		],
	},
	{
		title: 'Idle Cost',
		subtitle: 'Cost when not in use',
		direction: 'lower is better',
		chart: [0.09, 0.3, 1],
		bars: [
			{
				label: 'Rivet Actor',
				value: '$0',
				highlight: true,
				note: 'Assumes Rivet Actors running on a serverless platform. Actors scale to zero with no idle infrastructure costs. Traditional container deployments may incur idle costs.',
			},
			{
				label: 'Virtual Machine',
				value: '~$5/mo',
				note: 'AWS EC2 t3.nano ($0.0052/hr compute + $1.60/mo for 20GB gp3 storage) running 24/7. t3.nano is the smallest available EC2 instance (512MB RAM).',
			},
			{
				label: 'Kubernetes Cluster',
				value: '~$85/mo',
				note: 'AWS EKS control plane ($73/mo) plus a single t3.nano worker node with 20GB gp3 storage, running 24/7. t3.nano is the smallest available EC2 instance (512MB RAM).',
			},
		],
	},
	{
		title: 'Horizontal Scale',
		subtitle: 'Maximum capacity',
		direction: 'higher is better',
		chart: [1, 0.45, 0.09],
		bars: [
			{
				label: 'Rivet Actors',
				value: 'Infinite',
				highlight: true,
				note: 'Rivet Actors scale linearly by adding nodes with no single cluster size limit.',
			},
			{
				label: 'Kubernetes',
				value: '~5k nodes',
				note: 'Kubernetes officially supports clusters of up to 5,000 nodes per the Kubernetes scalability documentation.',
			},
			{ label: 'Postgres', value: '1 primary' },
		],
	},
	{
		title: 'Multi-Region',
		subtitle: 'Deploy actors close to your users',
		direction: 'lower is better',
		chart: [1, 0.18],
		bars: [
			{
				label: 'Rivet',
				value: 'Global edge network',
				highlight: true,
				note: 'Rivet automatically spawns actors near your users and handles routing across regions for a seamless edge network.',
			},
			{
				label: 'Traditional Deployment',
				value: '1 region',
			},
		],
	},
];

export const BenchmarksSection = () => {
	return (
		<section className='border-t border-ink/10 px-6 py-16 lg:py-24'>
			<div className='mx-auto w-full max-w-7xl'>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className='mb-10'
				>
					<Eyebrow index='03' label='Measurements' className='mb-4' />
					<h2 className={SECTION_H2_CLASS}>
						How Actors Compare
					</h2>
					<p className={SUBTITLE_CLASS}>
						Rivet Actors vs. traditional infrastructure.
					</p>
				</motion.div>

				<div className='grid grid-cols-1 gap-6 md:grid-cols-2'>
					{benchmarks.map((card, idx) => {
						const accentIdx = card.bars.findIndex((bar) => bar.highlight);
						const rivetStat = card.bars[accentIdx]?.value ?? '';
						return (
							<motion.div
								key={card.title}
								initial={{ opacity: 0, y: 16 }}
								whileInView={{ opacity: 1, y: 0 }}
								viewport={{ once: true }}
								transition={{ duration: 0.4, delay: idx * 0.05 }}
							>
								<InkPanel className='h-full'>
									<div className='flex h-full flex-col p-6'>
										<div className='mb-5 flex items-start justify-between gap-4'>
											<span className='font-mono text-[11px] uppercase tracking-[0.16em] text-sage'>
												{card.title}
											</span>
											<span className='flex items-center gap-1 font-mono text-[10px] uppercase tracking-wider text-cream/40'>
												{card.direction === 'lower is better' ? (
													<ArrowDown className='h-3 w-3' />
												) : (
													<ArrowUp className='h-3 w-3' />
												)}
												{card.direction}
											</span>
										</div>

										<div className='flex items-end justify-between gap-6'>
											<div className='min-w-0'>
												<div className='text-2xl font-medium leading-tight text-cream md:text-3xl'>
													{rivetStat}
												</div>
												<div className='mt-1 text-sm text-cream/55'>{card.subtitle}</div>
											</div>
											<PixelGridChart
												values={card.chart}
												rows={7}
												accentColumn={accentIdx}
												className='h-20 w-auto flex-shrink-0'
											/>
										</div>

										<div className='mt-6 flex flex-1 flex-col justify-end gap-2 border-t border-cream/10 pt-4'>
											{card.bars.map((bar) => (
												<div key={bar.label} className='flex items-center justify-between gap-4 text-xs'>
													<span
														className={`inline-flex items-center ${
															bar.highlight ? 'text-cream' : 'text-cream/55'
														}`}
													>
														{bar.label}
														{bar.note && <Tooltip note={bar.note} />}
													</span>
													<span className={bar.highlight ? 'font-medium text-sage' : 'text-cream/55'}>
														{bar.value}
													</span>
												</div>
											))}
										</div>
									</div>
								</InkPanel>
							</motion.div>
						);
					})}
				</div>

				<p className='mt-8 font-mono text-xs text-ink-faint'>
					Methodology — figures are directional, measured on commodity AWS infrastructure. Hover each entry for the measurement setup.
				</p>
			</div>
		</section>
	);
};
