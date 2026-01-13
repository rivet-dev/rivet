'use client';

import { Check, ArrowRight } from 'lucide-react';
import { motion } from 'framer-motion';
import Image from 'next/image';
import Link from 'next/link';

import imgYC from '@/images/logos/yc.svg';
import imgA16z from '@/images/logos/a16z.svg';

const FeatureCard = ({ title, description, index }: { title: string; description: string; index: number }) => (
	<motion.div
		initial={{ opacity: 0, y: 20 }}
		whileInView={{ opacity: 1, y: 0 }}
		viewport={{ once: true }}
		transition={{ duration: 0.5, delay: index * 0.1 }}
		className="group relative flex flex-col overflow-hidden rounded-2xl border border-white/10 bg-white/[0.02] p-6 backdrop-blur-sm transition-all duration-500 hover:border-white/20 hover:bg-white/[0.04]"
	>
		{/* Top Shine */}
		<div className="absolute left-0 right-0 top-0 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent opacity-0 transition-opacity group-hover:opacity-100" />

		<h3 className="mb-3 text-lg font-medium tracking-tight text-white">{title}</h3>
		<p className="text-sm leading-relaxed text-zinc-400">{description}</p>
	</motion.div>
);

const BenefitItem = ({ children, delay = 0 }: { children: React.ReactNode; delay?: number }) => (
	<motion.div
		initial={{ opacity: 0, x: -10 }}
		whileInView={{ opacity: 1, x: 0 }}
		viewport={{ once: true }}
		transition={{ duration: 0.4, delay }}
		className="flex items-start gap-3"
	>
		<div className="mt-1 flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-full bg-[#FF4500]/10">
			<Check className="h-3 w-3 text-[#FF4500]" />
		</div>
		<span className="text-zinc-300">{children}</span>
	</motion.div>
);

const StepItem = ({ number, title, description, delay = 0 }: { number: number; title: string; description: string; delay?: number }) => (
	<motion.div
		initial={{ opacity: 0, y: 20 }}
		whileInView={{ opacity: 1, y: 0 }}
		viewport={{ once: true }}
		transition={{ duration: 0.5, delay }}
		className="flex gap-4"
	>
		<div className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-[#FF4500]/10 text-sm font-medium text-[#FF4500]">
			{number}
		</div>
		<div>
			<h3 className="mb-2 font-medium text-white">{title}</h3>
			<p className="text-sm leading-relaxed text-zinc-400">{description}</p>
		</div>
	</motion.div>
);

export default function StartupsPage() {
	const benefits = [
		{ title: '50% off for 12 months', description: '50% off Team plan only' },
		{ title: 'Priority Slack support', description: 'Direct access to our engineering team for fast answers and guidance' },
		{ title: 'Architecture review', description: '1-on-1 session with our team to optimize your actor architecture' },
	];

	return (
		<div className="min-h-screen bg-black font-sans text-zinc-300 selection:bg-[#FF4500]/30 selection:text-orange-200">
			<main className="mx-auto w-full max-w-[1500px] px-4 md:px-8">
				{/* Hero Section */}
				<section className="relative overflow-hidden pb-8 pt-32 sm:pb-10 md:pt-48">
					<div className="pointer-events-none absolute left-1/2 top-0 h-[500px] w-[1000px] -translate-x-1/2 rounded-full bg-[#FF4500]/[0.03] blur-[100px]" />

					<div className="relative z-10 mx-auto max-w-[1200px] px-6 lg:px-8">
						<div className="text-center">
							<motion.h1
								initial={{ opacity: 0, y: 20 }}
								animate={{ opacity: 1, y: 0 }}
								transition={{ duration: 0.5, delay: 0.1 }}
								className="mb-6 text-4xl font-medium leading-[1.1] tracking-tighter text-white md:text-5xl lg:text-6xl"
							>
								Built for Demo Day
								<br className="hidden sm:block" />
								<span className="bg-gradient-to-b from-zinc-200 to-zinc-500 bg-clip-text text-transparent">
									and Beyond
								</span>
							</motion.h1>
						</div>
					</div>
				</section>

				{/* Deal Banner */}
				<section className="py-12 sm:py-16">
					<motion.div
						initial={{ opacity: 0, scale: 0.95 }}
						whileInView={{ opacity: 1, scale: 1 }}
						viewport={{ once: true }}
						transition={{ duration: 0.6, ease: [0.21, 0.47, 0.32, 0.98] }}
						className="mx-auto max-w-4xl px-6 lg:px-8"
					>
						<div className="relative overflow-hidden rounded-2xl border border-[#FF4500]/30 bg-gradient-to-b from-[#FF4500]/10 to-transparent p-8 shadow-[0_0_50px_-12px_rgba(255,69,0,0.15)]">
							{/* Top Shine */}
							<div className="absolute left-0 right-0 top-0 h-[1px] bg-gradient-to-r from-transparent via-[#FF4500]/50 to-transparent" />

							<div className="text-center">
								<motion.h2
									initial={{ opacity: 0, y: 10 }}
									whileInView={{ opacity: 1, y: 0 }}
									viewport={{ once: true }}
									transition={{ duration: 0.4, delay: 0.3 }}
									className="mb-4 text-2xl font-medium tracking-tight text-white md:text-3xl"
								>
									50% off Rivet Cloud for 12 months
								</motion.h2>

								<motion.p
									initial={{ opacity: 0, y: 10 }}
									whileInView={{ opacity: 1, y: 0 }}
									viewport={{ once: true }}
									transition={{ duration: 0.4, delay: 0.4 }}
									className="mx-auto mb-6 max-w-2xl text-lg leading-relaxed text-zinc-400"
								>
									As{' '}
									<span className="inline-flex items-center gap-1.5 whitespace-nowrap rounded-full border border-white/10 bg-white/5 px-2.5 py-0.5 text-sm text-zinc-300 align-middle">
										<Image src={imgYC} alt="Y Combinator" className="h-4 w-auto" />
										<span>YC W23</span>
									</span>
									{' '}and{' '}
									<span className="inline-flex items-center gap-1.5 whitespace-nowrap rounded-full border border-white/10 bg-white/5 px-2.5 py-0.5 text-sm text-zinc-300 align-middle">
										<Image src={imgA16z} alt="a16z" className="h-3 w-auto" />
										<span>a16z SR002</span>
									</span>
									{' '}alumni, we're offering fellow YC and Speedrun companies exclusive pricing to help you ship faster.
								</motion.p>

								<motion.div
									initial={{ opacity: 0, y: 10 }}
									whileInView={{ opacity: 1, y: 0 }}
									viewport={{ once: true }}
									transition={{ duration: 0.4, delay: 0.5 }}
								>
									<Link
										href="https://forms.gle/J8USsTND8NAKJ18W9"
										target="_blank"
										rel="noopener noreferrer"
										className="font-v2 inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black subpixel-antialiased shadow-sm transition-colors hover:bg-zinc-200"
									>
										Claim the deal
										<ArrowRight className="h-4 w-4" />
									</Link>
								</motion.div>
							</div>
						</div>
					</motion.div>
				</section>

				<div className="mx-auto max-w-4xl px-6 lg:px-8">
					{/* What You Get */}
					<section className="py-16 sm:py-20">
						<motion.div
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5 }}
							className="mb-12"
						>
							<h2 className="mb-4 text-2xl font-medium tracking-tight text-white md:text-3xl">What you get</h2>
							<p className="text-lg leading-relaxed text-zinc-400">
								Everything you need to build and scale stateful workloads at startup speed.
							</p>
						</motion.div>

						<div className="grid gap-4 md:grid-cols-3">
							{benefits.map((benefit, idx) => (
								<FeatureCard key={idx} title={benefit.title} description={benefit.description} index={idx} />
							))}
						</div>
					</section>

					{/* Eligibility */}
					<section className="py-16 sm:py-20">
						<motion.h2
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5 }}
							className="mb-8 text-2xl font-medium tracking-tight text-white md:text-3xl"
						>
							Eligibility
						</motion.h2>

						<div className="grid gap-4 md:grid-cols-3">
							{[
								'Current YC company or YC alumni',
								'OR current a16z Speedrun company or Speedrun alumni',
								'New Rivet Cloud customer',
							].map((item, idx) => (
								<motion.div
									key={idx}
									initial={{ opacity: 0, y: 20 }}
									whileInView={{ opacity: 1, y: 0 }}
									viewport={{ once: true }}
									transition={{ duration: 0.5, delay: idx * 0.1 }}
									className="group relative flex items-center gap-3 overflow-hidden rounded-2xl border border-white/10 bg-white/[0.02] p-6 backdrop-blur-sm transition-all duration-500 hover:border-white/20 hover:bg-white/[0.04]"
								>
									<div className="absolute left-0 right-0 top-0 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent opacity-0 transition-opacity group-hover:opacity-100" />
									<div className="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-full bg-[#FF4500]/10">
										<Check className="h-3 w-3 text-[#FF4500]" />
									</div>
									<span className="text-sm text-zinc-300">{item}</span>
								</motion.div>
							))}
						</div>
					</section>

					{/* How to Claim */}
					<section className="py-16 sm:py-20">
						<motion.h2
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5 }}
							className="mb-8 text-2xl font-medium tracking-tight text-white md:text-3xl"
						>
							How to claim
						</motion.h2>

						<div className="grid gap-4 md:grid-cols-3">
							{[
								{ number: 1, title: 'Reach out', description: 'Contact us through the form below or find the deal on Bookface/Speedrun portal' },
								{ number: 2, title: 'Verify your company', description: "We'll confirm your YC or Speedrun affiliation" },
								{ number: 3, title: 'Start building', description: 'Get your discount applied and start shipping with Rivet' },
							].map((step, idx) => (
								<motion.div
									key={idx}
									initial={{ opacity: 0, y: 20 }}
									whileInView={{ opacity: 1, y: 0 }}
									viewport={{ once: true }}
									transition={{ duration: 0.5, delay: idx * 0.1 }}
									className="group relative overflow-hidden rounded-2xl border border-white/10 bg-white/[0.02] p-6 backdrop-blur-sm transition-all duration-500 hover:border-white/20 hover:bg-white/[0.04]"
								>
									<div className="absolute left-0 right-0 top-0 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent opacity-0 transition-opacity group-hover:opacity-100" />
									<div className="mb-3 flex h-8 w-8 items-center justify-center rounded-full bg-[#FF4500]/10 text-sm font-medium text-[#FF4500]">
										{step.number}
									</div>
									<h3 className="mb-2 font-medium text-white">{step.title}</h3>
									<p className="text-sm leading-relaxed text-zinc-400">{step.description}</p>
								</motion.div>
							))}
						</div>
					</section>

					{/* CTA */}
					<section className="relative overflow-hidden border-t border-white/10 px-6 py-32 text-center">
						<motion.div
							animate={{ opacity: [0.3, 0.5, 0.3] }}
							transition={{ duration: 4, repeat: Infinity }}
							className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_center,_var(--tw-gradient-stops))] from-[#FF4500]/10 via-transparent to-transparent opacity-50"
						/>
						<div className="relative z-10 mx-auto max-w-3xl">
							<motion.div
								initial={{ opacity: 0, y: 20 }}
								whileInView={{ opacity: 1, y: 0 }}
								viewport={{ once: true }}
								transition={{ duration: 0.5 }}
							>
								<h2 className="mb-8 text-4xl font-medium tracking-tight text-white md:text-5xl">
									Ready to build?
								</h2>

								<motion.div
									initial={{ opacity: 0, y: 20 }}
									whileInView={{ opacity: 1, y: 0 }}
									viewport={{ once: true }}
									transition={{ duration: 0.5, delay: 0.2 }}
									className="flex flex-col items-center justify-center gap-4 sm:flex-row"
								>
									<Link
										href="https://forms.gle/J8USsTND8NAKJ18W9"
										target="_blank"
										rel="noopener noreferrer"
										className="font-v2 inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black subpixel-antialiased shadow-sm transition-colors hover:bg-zinc-200"
									>
										Claim the deal
										<ArrowRight className="h-4 w-4" />
									</Link>
								</motion.div>

								<p className="mt-10 text-zinc-500">
									Questions?{' '}
									<Link href="/support" className="text-zinc-300 transition-colors hover:text-white hover:underline">
										Contact us
									</Link>
								</p>
							</motion.div>
						</div>
					</section>
				</div>
			</main>
		</div>
	);
}
