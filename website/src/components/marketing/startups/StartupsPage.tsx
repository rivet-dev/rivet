'use client';

import { useState, useEffect } from 'react';
import { Check, ArrowRight, ChevronDown } from 'lucide-react';
import imgYC from '@/images/logos/yc.svg';
import imgA16z from '@/images/logos/a16z.svg';

// Demo day dates - update these when new dates are announced
const YC_DEMO_DAY = new Date('2026-03-24T00:00:00-07:00');
const A16Z_DEMO_DAY = new Date('2026-04-14T00:00:00-07:00');

function getTimeRemaining(targetDate: Date) {
	const now = new Date();
	const diff = targetDate.getTime() - now.getTime();

	if (diff <= 0) return null;

	const totalHours = Math.floor(diff / (1000 * 60 * 60));
	const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));
	const seconds = Math.floor((diff % (1000 * 60)) / 1000);

	return { totalHours, minutes, seconds };
}

function DemoCountdown() {
	const [ycCountdown, setYcCountdown] = useState(getTimeRemaining(YC_DEMO_DAY));
	const [a16zCountdown, setA16zCountdown] = useState(getTimeRemaining(A16Z_DEMO_DAY));

	useEffect(() => {
		const interval = setInterval(() => {
			setYcCountdown(getTimeRemaining(YC_DEMO_DAY));
			setA16zCountdown(getTimeRemaining(A16Z_DEMO_DAY));
		}, 1000);

		return () => clearInterval(interval);
	}, []);

	const formatCountdown = (countdown: { totalHours: number; minutes: number; seconds: number } | null) => {
		if (!countdown) return 'Passed';
		const h = String(countdown.totalHours).padStart(2, '0');
		const m = String(countdown.minutes).padStart(2, '0');
		const s = String(countdown.seconds).padStart(2, '0');
		return `${h}:${m}:${s}`;
	};

	return (
		<div className="flex flex-wrap gap-6 font-mono text-xs">
			{ycCountdown && (
				<div className="flex items-center gap-2">
					<span className="text-zinc-500">YC Demo Day</span>
					<span className="text-zinc-300">{formatCountdown(ycCountdown)}</span>
				</div>
			)}
			{a16zCountdown && (
				<div className="flex items-center gap-2">
					<span className="text-zinc-500">a16z Demo Day</span>
					<span className="text-zinc-300">{formatCountdown(a16zCountdown)}</span>
				</div>
			)}
		</div>
	);
}

interface CollapsibleSectionProps {
	title: string;
	children: React.ReactNode;
	defaultOpen?: boolean;
}

function CollapsibleSection({ title, children, defaultOpen = false }: CollapsibleSectionProps) {
	const [isOpen, setIsOpen] = useState(defaultOpen);

	return (
		<div className="border-t border-white/10 px-6">
			<div className="mx-auto w-full max-w-7xl">
				<button
					onClick={() => setIsOpen(!isOpen)}
					className="flex h-28 w-full items-center justify-between text-left"
				>
					<h2 className="text-2xl font-normal tracking-tight text-white md:text-4xl">
						{title}
					</h2>
					<ChevronDown
						className={`h-6 w-6 text-zinc-500 transition-transform duration-200 ${
							isOpen ? 'rotate-180' : ''
						}`}
					/>
				</button>
				<div
					className={`grid transition-all duration-300 ease-in-out ${
						isOpen ? 'grid-rows-[1fr] opacity-100 pb-16' : 'grid-rows-[0fr] opacity-0'
					}`}
				>
					<div className="overflow-hidden">
						{children}
					</div>
				</div>
			</div>
		</div>
	);
}

interface StartupsPageProps {
	foundersImage: string;
	speedrunImage: string;
}

export default function StartupsPage({ foundersImage, speedrunImage }: StartupsPageProps) {
	const benefits = [
		{ title: '50% off for 12 months', description: '50% off the Team plan' },
		{ title: 'Priority Slack support', description: 'Direct access to our engineering team for fast answers and guidance' },
		{ title: 'Architecture review', description: '1-on-1 session with our team to optimize your actor architecture' },
	];

	const eligibility = [
		'Current YC company or YC alumni',
		'OR current a16z Speedrun company or Speedrun alumni',
		'New Rivet Cloud customer',
	];

	const steps = [
		{ number: 1, title: 'Reach out', description: 'Contact us through the form below or find the deal on Bookface/Speedrun portal' },
		{ number: 2, title: 'Verify your company', description: "We'll confirm your YC or Speedrun affiliation" },
		{ number: 3, title: 'Start building', description: 'Get your discount applied and start shipping with Rivet' },
	];

	return (
		<div className="min-h-screen bg-black font-sans text-zinc-300 selection:bg-[#FF4500]/30 selection:text-orange-200">
			{/* Hero Section */}
			<section className="relative flex min-h-screen flex-col overflow-hidden">
				{/* Centered content */}
				<div className="flex flex-1 flex-col justify-center px-6">
					<div className="mx-auto w-full max-w-7xl">
						<div className="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-12 lg:gap-20">
							<div className="max-w-xl">
								<h1 className="mb-6 text-4xl font-normal leading-[1.1] tracking-tight text-white md:text-6xl">
									Built for Demo Day and Beyond
								</h1>
								<p className="text-base leading-relaxed text-zinc-500">
									As{' '}
									<span className="inline-flex items-center gap-1.5 whitespace-nowrap rounded-full border border-white/10 bg-white/5 px-2.5 py-0.5 text-sm text-zinc-300 align-middle">
										<img src={imgYC.src} alt="Y Combinator logo" width={16} height={16} className="h-4 w-auto" loading="eager" decoding="async" />
										<span>YC W23</span>
									</span>
									{' '}and{' '}
									<span className="inline-flex items-center gap-1.5 whitespace-nowrap rounded-full border border-white/10 bg-white/5 px-2.5 py-0.5 text-sm text-zinc-300 align-middle">
										<img src={imgA16z.src} alt="Andreessen Horowitz (a16z) logo" width={16} height={12} className="h-3 w-auto" loading="eager" decoding="async" />
										<span>a16z SR002</span>
									</span>
									{' '}alumni, we're offering fellow YC and Speedrun companies pricing and support to ship faster.
								</p>
							</div>
							{/* Desktop: Overlapping photos */}
							<div className="hidden lg:block flex-shrink-0 relative w-[500px] h-[400px]">
								<div className="absolute top-0 left-0 w-[320px] h-[240px] overflow-hidden rounded-lg border border-white/10">
									<img
										src={foundersImage}
										alt="Rivet founders Nathan Flurry and Nicholas Kissel at Y Combinator W23 Demo Day"
										width={320}
										height={240}
										loading="eager"
										decoding="async"
										className="w-full h-full object-cover"
									/>
								</div>
								<div className="absolute bottom-0 right-0 w-[320px] h-[240px] overflow-hidden rounded-lg border border-white/10">
									<img
										src={speedrunImage}
										alt="Andreessen Horowitz a16z Speedrun SR002 cohort presentation"
										width={320}
										height={240}
										loading="lazy"
										decoding="async"
										className="w-full h-full object-cover"
									/>
								</div>
							</div>
							{/* Mobile: Stacked photos */}
							<div className="flex flex-col gap-4 lg:hidden">
								<div className="w-full h-[200px] overflow-hidden rounded-lg border border-white/10">
									<img
										src={foundersImage}
										alt="Rivet founders Nathan Flurry and Nicholas Kissel at Y Combinator W23 Demo Day"
										loading="eager"
										decoding="async"
										className="w-full h-full object-cover"
									/>
								</div>
								<div className="w-full h-[200px] overflow-hidden rounded-lg border border-white/10">
									<img
										src={speedrunImage}
										alt="Andreessen Horowitz a16z Speedrun SR002 cohort presentation"
										loading="lazy"
										decoding="async"
										className="w-full h-full object-cover"
									/>
								</div>
							</div>
						</div>
					</div>
				</div>

				{/* Bottom section */}
				<div className="absolute bottom-0 left-0 right-0 px-6 pb-24">
					<div className="mx-auto w-full max-w-7xl">
						<div className="mb-6">
							<DemoCountdown />
						</div>
						<div className="mb-8 h-px w-full bg-white/10" />
						<div className="flex flex-col md:flex-row md:items-center md:justify-between gap-6">
							<div>
								<h2 className="text-base font-normal text-white">
									50% off Rivet Cloud for 12 months
								</h2>
								<p className="mt-1 text-sm text-zinc-500">
									Everything you need to build and scale stateful workloads at startup speed.
								</p>
							</div>
							<a
								href="https://forms.gle/J8USsTND8NAKJ18W9"
								target="_blank"
								rel="noopener noreferrer"
								className="selection-dark inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200"
							>
								Claim the deal
								<ArrowRight className="h-4 w-4" />
							</a>
						</div>
					</div>
				</div>
			</section>

			{/* What You Get */}
			<CollapsibleSection title="What you get" defaultOpen>
				<p className="mb-12 max-w-xl text-base leading-relaxed text-zinc-500">
					Everything you need to build and scale stateful workloads at startup speed.
				</p>
				<div className="grid grid-cols-1 gap-8 md:grid-cols-3">
					{benefits.map((benefit, idx) => (
						<div key={idx} className="flex flex-col border-t border-white/10 pt-6">
							<h3 className="mb-2 text-base font-normal text-white">{benefit.title}</h3>
							<p className="text-sm leading-relaxed text-zinc-500">{benefit.description}</p>
						</div>
					))}
				</div>
			</CollapsibleSection>

			{/* Eligibility */}
			<CollapsibleSection title="Eligibility">
				<div className="grid grid-cols-1 gap-4 md:grid-cols-3">
					{eligibility.map((item, idx) => (
						<div key={idx} className="flex items-center gap-3 rounded-md border border-white/10 p-4">
							<div className="flex h-5 w-5 flex-shrink-0 items-center justify-center">
								<Check className="h-4 w-4 text-zinc-500" />
							</div>
							<span className="text-sm text-zinc-300">{item}</span>
						</div>
					))}
				</div>
			</CollapsibleSection>

			{/* How to Claim */}
			<CollapsibleSection title="How to claim">
				<div className="grid grid-cols-1 gap-8 md:grid-cols-3">
					{steps.map((step, idx) => (
						<div key={idx} className="flex flex-col border-t border-white/10 pt-6">
							<div className="mb-3 flex h-6 w-6 items-center justify-center rounded-full border border-white/10 text-xs text-zinc-500">
								{step.number}
							</div>
							<h3 className="mb-2 text-base font-normal text-white">{step.title}</h3>
							<p className="text-sm leading-relaxed text-zinc-500">{step.description}</p>
						</div>
					))}
				</div>
			</CollapsibleSection>

			{/* CTA */}
			<div className="border-t border-white/10 py-24 px-6">
				<div className="mx-auto w-full max-w-7xl text-center">
					<h2 className="mb-6 text-2xl font-normal tracking-tight text-white md:text-4xl">
						Ready to build?
					</h2>
					<div className="flex flex-col items-center justify-center gap-4 sm:flex-row">
						<a
							href="https://forms.gle/J8USsTND8NAKJ18W9"
							target="_blank"
							rel="noopener noreferrer"
							className="selection-dark inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200"
						>
							Claim the deal
							<ArrowRight className="h-4 w-4" />
						</a>
					</div>
					<p className="mt-8 text-sm text-zinc-500">
						Questions?{' '}
						<a href="/support" className="text-zinc-300 transition-colors hover:text-white">
							Contact us
						</a>
					</p>
				</div>
			</div>
		</div>
	);
}
