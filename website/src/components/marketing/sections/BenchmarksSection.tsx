'use client';

import { motion } from 'framer-motion';
import { SECTION_H2_CLASS } from '../typography';
import { BenchCard, BenchInfoTooltip, type BenchRowEntry } from '../bench/BenchCard';

interface BarEntry {
	label: string;
	value: string;
	highlight?: boolean;
	/** Hover note explaining the benchmark context. */
	note?: string;
}

interface BenchmarkCard {
	title: string;
	/** Headline stat. Defaults to the highlighted entry's value when omitted. */
	hero?: string;
	/** Comparative shown next to the headline (e.g. "faster"). Only set where the
	    competitor/Rivet ratio is a finite number; division-by-zero metrics
	    (0ms reads, $0 idle) and qualitative ones keep their standout value. */
	verb?: string;
	bars: BarEntry[];
	/** Optional caveat pinned to the card foot. */
	note?: string;
}

const benchmarks: BenchmarkCard[] = [
	{
		title: 'Cold Start',		hero: '~300x',
		verb: 'faster',
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
		title: 'Memory Per Instance',		hero: '~80,000x',
		verb: 'smaller',
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
		title: 'Read Latency',		bars: [
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
		title: 'Idle Cost',		note: 'Actors scale to zero with no idle infrastructure.',
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
		title: 'Horizontal Scale',		bars: [
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
		title: 'Multi-Region',		hero: 'Global',
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
					<h2 className={SECTION_H2_CLASS}>How Actors Compare</h2>
				</motion.div>

				<div className='grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3'>
					{benchmarks.map((card) => {
						const accent = card.bars.find((bar) => bar.highlight);
						const rows: BenchRowEntry[] = card.bars.map((bar) => ({
							label: bar.note ? (
								<>
									{bar.label}
									<BenchInfoTooltip>{bar.note}</BenchInfoTooltip>
								</>
							) : (
								bar.label
							),
							value: bar.value,
							highlight: bar.highlight,
						}));

						return (
							<BenchCard
								key={card.title}
								title={card.title}
								statNote={card.hero ?? accent?.value ?? ''}
								verb={card.verb}
								rows={rows}
								note={card.note}
							/>
						);
					})}
				</div>

			</div>
		</section>
	);
};
