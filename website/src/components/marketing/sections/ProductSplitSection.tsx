'use client';

import { Database, Globe, Infinity, Layers, Wifi, GitBranch, ListOrdered, Clock, Shield, FolderOpen, Code, ArrowRight } from 'lucide-react';
import { motion } from 'framer-motion';
import { SECTION_H2_CLASS, SUBTITLE_CLASS } from '../typography';
import agentosLogoUrl from '@/images/products/agentos-logo.svg';

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

const agentOSFeatures = [
	{
		icon: Clock,
		title: '~6ms coldstart, 22 MB RAM',
		description: 'Near-zero cold start with minimal overhead.',
	},
	{
		icon: Layers,
		title: 'An order of magnitude cheaper',
		description: 'V8 isolates + Wasm instead of full VM sandboxes.',
	},
	{
		icon: Code,
		title: 'Embed in your backend',
		description: 'Your APIs. Your toolchains. No complex agent auth.',
	},
	{
		icon: FolderOpen,
		title: 'Mount anything as a file system',
		description: 'S3, SQLite, Google Drive, or the host file system.',
	},
	{
		icon: Shield,
		title: 'Granular security',
		description: 'V8 isolates + WebAssembly. Configurable policies.',
	},
	{
		icon: Globe,
		title: 'Your laptop, your infra, or on-prem',
		description: 'Rivet Cloud, Railway, Vercel, Kubernetes, or on-prem.',
	},
];

const RivetActorIcon = ({ className }: { className?: string }) => (
	<svg width="24" height="24" viewBox="0 0 176 173" className={className}>
		<g transform="translate(-32928.8,-28118.2)">
			<g transform="matrix(0.941176,0,0,0.925134,2119.4,2323.67)">
				<g clipPath="url(#_clip1)">
					<g transform="matrix(1.0625,0,0,1.08092,32936.6,27881.1)">
						<path d="M164.529,52.792L164.529,120.844C164.529,145.347 144.635,165.241 120.132,165.241L52.08,165.241C27.577,165.241 7.683,145.347 7.683,120.844L7.683,52.792C7.683,28.289 27.577,8.395 52.08,8.395L120.132,8.395C144.635,8.395 164.529,28.289 164.529,52.792Z" style={{ fill: 'none', stroke: 'currentColor', strokeWidth: '15.18px' }} />
					</g>
					<g transform="matrix(1.0625,0,0,1.08092,32737,27881.7)">
						<path d="M164.529,52.792L164.529,120.844C164.529,145.347 144.635,165.241 120.132,165.241L52.08,165.241C27.577,165.241 7.683,145.347 7.683,120.844L7.683,52.792C7.683,28.289 27.577,8.395 52.08,8.395L120.132,8.395C144.635,8.395 164.529,28.289 164.529,52.792Z" style={{ fill: 'none', stroke: 'currentColor', strokeWidth: '15.18px' }} />
					</g>
				</g>
			</g>
		</g>
		<g transform="translate(-32928.8,-28118.2)">
			<g transform="matrix(0.941176,0,0,0.925134,2119.4,2323.67)">
				<g clipPath="url(#_clip1)">
					<g transform="matrix(1.0625,0,0,1.08092,-2251.86,-2261.21)">
						<g transform="translate(32930.7,27886.2)">
							<path d="M104.323,87.121C104.584,85.628 105.665,84.411 107.117,83.977C108.568,83.542 110.14,83.965 111.178,85.069C118.49,92.847 131.296,106.469 138.034,113.637C138.984,114.647 139.343,116.076 138.983,117.415C138.623,118.754 137.595,119.811 136.267,120.208C127.471,122.841 111.466,127.633 102.67,130.266C101.342,130.664 99.903,130.345 98.867,129.425C97.83,128.504 97.344,127.112 97.582,125.747C99.274,116.055 102.488,97.637 104.323,87.121Z" style={{ fill: 'currentColor' }} />
						</g>
						<g transform="translate(32930.7,27886.2)">
							<path d="M69.264,88.242L79.739,106.385C82.629,111.392 80.912,117.803 75.905,120.694L57.762,131.168C52.755,134.059 46.344,132.341 43.453,127.335L32.979,109.192C30.088,104.185 31.806,97.774 36.813,94.883L54.956,84.408C59.962,81.518 66.374,83.236 69.264,88.242Z" style={{ fill: 'currentColor' }} />
						</g>
						<g transform="translate(32930.7,27886.2)">
							<path d="M86.541,79.464C98.111,79.464 107.49,70.084 107.49,58.514C107.49,46.944 98.111,37.565 86.541,37.565C74.971,37.565 65.591,46.944 65.591,58.514C65.591,70.084 74.971,79.464 86.541,79.464Z" style={{ fill: 'currentColor' }} />
						</g>
					</g>
				</g>
			</g>
		</g>
	</svg>
);

interface ProductCardProps {
	icon: React.ReactNode;
	title: string;
	tagline: string;
	docsHref: string;
	detailsHref: string;
	features: { icon: typeof Database; title: string; description: string }[];
	delay: number;
	external?: boolean;
}

const ProductCard = ({ icon, title, tagline, docsHref, detailsHref, features, delay, external }: ProductCardProps) => (
	<motion.div
		initial={{ opacity: 0, y: 20 }}
		whileInView={{ opacity: 1, y: 0 }}
		viewport={{ once: true }}
		transition={{ duration: 0.5, delay }}
		className='flex flex-col border border-ink/10 bg-white/55 p-6 md:p-8'
	>
		<div className='mb-4 flex items-center gap-3'>
			{icon}
			<h3 className='text-xl font-medium text-ink'>{title}</h3>
		</div>

		<p className='mb-6 text-sm leading-relaxed text-ink-soft'>
			{tagline}
		</p>

		<div className='mb-8 flex flex-wrap gap-3'>
			<a
				href={docsHref}
				target={external ? '_blank' : undefined}
				rel={external ? 'noopener noreferrer' : undefined}
				className='inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-ink px-4 py-2 text-sm font-medium text-cream transition-colors hover:bg-ink/85'
			>
				Documentation
			</a>
			<a
				href={detailsHref}
				target={external ? '_blank' : undefined}
				rel={external ? 'noopener noreferrer' : undefined}
				className='inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-ink/20 px-4 py-2 text-sm text-ink-soft transition-colors hover:border-ink/40 hover:text-ink'
			>
				Details
				<ArrowRight className='h-3.5 w-3.5' />
			</a>
		</div>

		<div className='grid grid-cols-2 gap-x-6 gap-y-5'>
			{features.map((feature) => {
				const Icon = feature.icon;
				return (
					<div key={feature.title} className='flex flex-col gap-1.5'>
						<div className='flex items-center gap-2'>
							<Icon className='h-3.5 w-3.5 text-olive' />
							<span className='text-sm font-medium text-ink'>{feature.title}</span>
						</div>
						<p className='text-xs leading-relaxed text-ink-faint'>{feature.description}</p>
					</div>
				);
			})}
		</div>
	</motion.div>
);

export const ProductSplitSection = () => (
	<section className='relative border-t border-ink/10 px-6 py-16 lg:py-24'>
		<div className='mx-auto w-full max-w-7xl'>
			<div className='mb-12 max-w-3xl'>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
				>
					<h2 className={SECTION_H2_CLASS}>Two products, one platform.</h2>
				</motion.div>
				<motion.p
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.05 }}
					className={SUBTITLE_CLASS}
				>
					Rivet Actors give agents and realtime apps durable, stateful compute in your existing Node.js or Bun backend. agentOS gives agents a portable OS to run in. Use them alone or together.
				</motion.p>
			</div>
			<div className='grid grid-cols-1 gap-6 lg:grid-cols-2'>
				<ProductCard
					icon={<RivetActorIcon className='text-ink' />}
					title='Actors'
					tagline='The primitive for stateful workloads.'
					docsHref='/docs'
					detailsHref='/actors'
					features={actorFeatures}
					delay={0}
				/>
				<ProductCard
					icon={<img src={agentosLogoUrl.src} alt='agentOS' className='h-6 w-6 invert' />}
					title='agentOS'
					tagline='A portable, lightweight in-process OS for agents. Open source, built on Wasm + V8.'
					docsHref='https://agentos-sdk.dev'
					detailsHref='https://agentos-sdk.dev'
					features={agentOSFeatures}
					delay={0.1}
					external
				/>
			</div>
		</div>
	</section>
);
