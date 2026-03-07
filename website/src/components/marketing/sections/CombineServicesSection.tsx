'use client';

import { motion, useInView } from 'framer-motion';
import { useRef } from 'react';
import { Database, Radio, Workflow, MessageSquare } from 'lucide-react';

const RivetActorIcon = ({ className }: { className?: string }) => (
	<svg width="24" height="24" viewBox="0 0 176 173" className={className}>
		<g transform="translate(-32928.8,-28118.2)">
			<g transform="matrix(0.941176,0,0,0.925134,2119.4,2323.67)">
				<g clipPath="url(#_clip1)">
					<g transform="matrix(1.0625,0,0,1.08092,32936.6,27881.1)">
						<path d="M164.529,52.792L164.529,120.844C164.529,145.347 144.635,165.241 120.132,165.241L52.08,165.241C27.577,165.241 7.683,145.347 7.683,120.844L7.683,52.792C7.683,28.289 27.577,8.395 52.08,8.395L120.132,8.395C144.635,8.395 164.529,28.289 164.529,52.792Z" style={{ fill: 'none', stroke: 'white', strokeWidth: '15.18px' }} />
					</g>
					<g transform="matrix(1.0625,0,0,1.08092,32737,27881.7)">
						<path d="M164.529,52.792L164.529,120.844C164.529,145.347 144.635,165.241 120.132,165.241L52.08,165.241C27.577,165.241 7.683,145.347 7.683,120.844L7.683,52.792C7.683,28.289 27.577,8.395 52.08,8.395L120.132,8.395C144.635,8.395 164.529,28.289 164.529,52.792Z" style={{ fill: 'none', stroke: 'white', strokeWidth: '15.18px' }} />
					</g>
				</g>
			</g>
		</g>
		<g transform="translate(-32928.8,-28118.2)">
			<g transform="matrix(0.941176,0,0,0.925134,2119.4,2323.67)">
				<g clipPath="url(#_clip1)">
					<g transform="matrix(1.0625,0,0,1.08092,-2251.86,-2261.21)">
						<g transform="translate(32930.7,27886.2)">
							<path d="M104.323,87.121C104.584,85.628 105.665,84.411 107.117,83.977C108.568,83.542 110.14,83.965 111.178,85.069C118.49,92.847 131.296,106.469 138.034,113.637C138.984,114.647 139.343,116.076 138.983,117.415C138.623,118.754 137.595,119.811 136.267,120.208C127.471,122.841 111.466,127.633 102.67,130.266C101.342,130.664 99.903,130.345 98.867,129.425C97.83,128.504 97.344,127.112 97.582,125.747C99.274,116.055 102.488,97.637 104.323,87.121Z" style={{ fill: 'white' }} />
						</g>
						<g transform="translate(32930.7,27886.2)">
							<path d="M69.264,88.242L79.739,106.385C82.629,111.392 80.912,117.803 75.905,120.694L57.762,131.168C52.755,134.059 46.344,132.341 43.453,127.335L32.979,109.192C30.088,104.185 31.806,97.774 36.813,94.883L54.956,84.408C59.962,81.518 66.374,83.236 69.264,88.242Z" style={{ fill: 'white' }} />
						</g>
						<g transform="translate(32930.7,27886.2)">
							<path d="M86.541,79.464C98.111,79.464 107.49,70.084 107.49,58.514C107.49,46.944 98.111,37.565 86.541,37.565C74.971,37.565 65.591,46.944 65.591,58.514C65.591,70.084 74.971,79.464 86.541,79.464Z" style={{ fill: 'white' }} />
						</g>
					</g>
				</g>
			</g>
		</g>
	</svg>
);

const services = [
	{ icon: Database, label: 'Redis', color: '#DC382D' },
	{ icon: Radio, label: 'Kafka', color: '#231F20' },
	{ icon: Workflow, label: 'Workflows', color: '#6366F1' },
	{ icon: MessageSquare, label: 'PubSub', color: '#10B981' },
];

export const CombineServicesSection = () => {
	const ref = useRef<HTMLDivElement>(null);
	const isInView = useInView(ref, { once: true, amount: 0.3 });

	return (
		<section className='border-t border-white/10 px-6 py-20 lg:py-32'>
			<div className='mx-auto max-w-7xl'>
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className='mb-16'
				>
					<h2 className='mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl'>
						Replace your infrastructure stack
					</h2>
					<p className='text-base leading-relaxed text-zinc-500'>
						One primitive replaces four separate services.
					</p>
				</motion.div>

				<div ref={ref} className='relative flex flex-col items-center gap-8 lg:flex-row lg:items-center lg:justify-center lg:gap-0'>
					{/* Left column: service cards */}
					<div className='flex flex-col gap-4 lg:w-64'>
						{services.map((service, idx) => {
							const Icon = service.icon;
							return (
								<motion.div
									key={service.label}
									initial={{ opacity: 0, x: -20 }}
									whileInView={{ opacity: 1, x: 0 }}
									viewport={{ once: true }}
									transition={{ duration: 0.4, delay: idx * 0.05 }}
									className='flex items-center gap-3 rounded-xl border border-white/10 bg-white/[0.03] px-5 py-4'
								>
									<Icon className='h-5 w-5 text-zinc-400' />
									<span className='text-sm font-medium text-white'>{service.label}</span>
								</motion.div>
							);
						})}
					</div>

					{/* Center: animated flow arrows */}
					<div className='hidden lg:block lg:w-64 lg:px-4'>
						<svg
							viewBox="0 0 200 260"
							fill="none"
							className='h-auto w-full'
						>
							{services.map((_, idx) => {
								const startY = 32.5 + idx * 65;
								const endY = 130;
								const d = `M 0 ${startY} C 80 ${startY}, 120 ${endY}, 200 ${endY}`;
								return (
									<motion.path
										key={idx}
										d={d}
										stroke='white'
										strokeOpacity={0.15}
										strokeWidth={1.5}
										fill='none'
										initial={{ pathLength: 0, opacity: 0 }}
										animate={isInView ? { pathLength: 1, opacity: 1 } : { pathLength: 0, opacity: 0 }}
										transition={{ duration: 0.8, delay: 0.3 + idx * 0.1, ease: 'easeInOut' }}
									/>
								);
							})}
							{/* Animated dots traveling along paths */}
							{services.map((_, idx) => {
								const startY = 32.5 + idx * 65;
								const endY = 130;
								const d = `M 0 ${startY} C 80 ${startY}, 120 ${endY}, 200 ${endY}`;
								return (
									<motion.circle
										key={`dot-${idx}`}
										r={3}
										fill='#FF4500'
										initial={{ opacity: 0 }}
										animate={isInView ? { opacity: [0, 1, 1, 0] } : { opacity: 0 }}
										transition={{ duration: 1.2, delay: 0.5 + idx * 0.1, ease: 'easeInOut' }}
									>
										<animateMotion
											dur="1.2s"
											begin={`${0.5 + idx * 0.1}s`}
											fill="freeze"
											path={d}
										/>
									</motion.circle>
								);
							})}
							{/* Arrow tip */}
							<motion.polygon
								points="194,124 200,130 194,136"
								fill='white'
								fillOpacity={0.3}
								initial={{ opacity: 0 }}
								animate={isInView ? { opacity: 1 } : { opacity: 0 }}
								transition={{ duration: 0.3, delay: 1.2 }}
							/>
						</svg>
					</div>

					{/* Mobile: simple arrow */}
					<div className='flex items-center justify-center lg:hidden'>
						<motion.div
							initial={{ opacity: 0 }}
							whileInView={{ opacity: 1 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.3 }}
						>
							<svg width="24" height="48" viewBox="0 0 24 48" fill="none">
								<path d="M12 0 L12 40 M6 34 L12 42 L18 34" stroke="white" strokeOpacity={0.3} strokeWidth={1.5} />
							</svg>
						</motion.div>
					</div>

					{/* Right column: Actors card */}
					<motion.div
						initial={{ opacity: 0, x: 20 }}
						whileInView={{ opacity: 1, x: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.4 }}
						className='lg:w-64'
					>
						<div className='flex items-center gap-3 rounded-xl border border-[#FF4500]/20 bg-[#FF4500]/[0.06] px-5 py-4'>
							<RivetActorIcon className='h-5 w-5' />
							<span className='text-sm font-medium text-white'>Actors</span>
						</div>
					</motion.div>
				</div>
			</div>
		</section>
	);
};
