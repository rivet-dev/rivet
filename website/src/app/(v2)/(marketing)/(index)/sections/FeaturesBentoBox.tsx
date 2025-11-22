"use client";

import { useRef } from "react";
import Link from "next/link";
import { Icon } from "@rivet-gg/icons";
import {
	faDatabase,
	faBolt,
	faMoon,
	faClock,
	faShieldHalved,
	faRocket,
	faCheckCircle,
} from "@rivet-gg/icons";

interface FeatureCardProps {
	title: string;
	description: string;
	href: string;
	className?: string;
	icon: any;
	variant?: "default" | "large" | "medium" | "small" | "wide" | "code";
}

function FeatureCard({
	title,
	description,
	href,
	className = "",
	icon,
	variant = "default",
}: FeatureCardProps) {
	const cardRef = useRef<HTMLAnchorElement>(null);

	const handleMouseMove = (e: React.MouseEvent<HTMLAnchorElement>) => {
		if (!cardRef.current) return;
		const card = cardRef.current;

		const iconContainer = card.querySelector('.icon-spotlight-container') as HTMLElement;
		if (!iconContainer) return;

		const iconRect = iconContainer.getBoundingClientRect();
		const x = ((e.clientX - iconRect.left) / iconRect.width) * 100;
		const y = ((e.clientY - iconRect.top) / iconRect.height) * 100;

		iconContainer.style.setProperty('--mouse-x', `${x}%`);
		iconContainer.style.setProperty('--mouse-y', `${y}%`);
	};

	if (variant === "large") {
		return (
			<Link
				ref={cardRef}
				href={href}
				className={`group relative block ${className}`}
				onMouseMove={handleMouseMove}
			>
				<div className="h-full border border-white/20 bg-white/[0.008] hover:border-white/30 hover:bg-white/[0.02] rounded-xl p-6 transition-all relative overflow-hidden flex flex-col">
					<div className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-300 pointer-events-none bg-gradient-to-b from-white/[0.04] via-white/[0.01] to-transparent" />

					<div className="relative z-10 flex-1 flex flex-col justify-between">
						<div>
							<div className="mb-4">
								<Icon icon={icon} className="w-12 h-12 text-white/70" />
							</div>
							<h3 className="text-2xl font-semibold text-white mb-3">
								{title}
							</h3>
							<p className="text-white/50 text-base leading-relaxed">
								{description}
							</p>
						</div>
					</div>
				</div>
			</Link>
		);
	}

	if (variant === "medium") {
		return (
			<Link
				ref={cardRef}
				href={href}
				className={`group relative block ${className}`}
				onMouseMove={handleMouseMove}
			>
				<div className="h-full border border-white/20 bg-white/[0.008] hover:border-white/30 hover:bg-white/[0.02] rounded-xl p-6 transition-colors relative overflow-hidden flex flex-col">
					<div className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-300 pointer-events-none bg-gradient-to-b from-white/[0.04] via-white/[0.01] to-transparent" />

					<div className="relative z-10 mb-4">
						<Icon icon={icon} className="w-10 h-10 text-white/70 mb-3" />
						<h3 className="text-lg font-semibold text-white mb-2">
							{title}
						</h3>
						<p className="text-white/50 text-sm leading-relaxed">
							{description}
						</p>
					</div>
				</div>
			</Link>
		);
	}

	if (variant === "small") {
		return (
			<Link
				ref={cardRef}
				href={href}
				className={`group relative block ${className}`}
				onMouseMove={handleMouseMove}
			>
				<div className="h-full border border-white/20 bg-white/[0.008] hover:border-white/30 hover:bg-white/[0.02] rounded-xl p-5 transition-colors relative overflow-hidden flex flex-col">
					<div className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-300 pointer-events-none bg-gradient-to-b from-white/[0.04] via-white/[0.01] to-transparent" />

					<div className="relative z-10">
						<div className="mb-3">
							<Icon icon={icon} className="w-8 h-8 text-white/60" />
						</div>
						<h3 className="text-base font-semibold text-white mb-2">
							{title}
						</h3>
						<p className="text-white/40 text-sm leading-relaxed">
							{description}
						</p>
					</div>
				</div>
			</Link>
		);
	}

	if (variant === "code") {
		return (
			<div className={`group relative block ${className}`}>
				<div className="h-full border border-white/10 bg-[#1e1e1e] hover:border-white/20 rounded-xl p-6 transition-colors relative overflow-hidden">
					<h3 className="text-lg font-semibold text-white mb-4">
						{title}
					</h3>
					<pre className="bg-[#282828] rounded-lg p-4 overflow-x-auto">
						<code className="text-sm text-[#d4d4d4] font-mono leading-relaxed whitespace-pre">
							<span className="text-[#c586c0]">const</span> <span className="text-[#9cdcfe]">counter</span> = <span className="text-[#dcdcaa]">actor</span>({`{`}{"\n"}
							{`  `}<span className="text-[#9cdcfe]">state</span>: {`{ `}<span className="text-[#9cdcfe]">count</span>: <span className="text-[#b5cea8]">0</span> {`}`},{`\n`}
							{`  `}<span className="text-[#9cdcfe]">actions</span>: {`{`}{"\n"}
							{`    `}<span className="text-[#dcdcaa]">increment</span>: (<span className="text-[#9cdcfe]">c</span>) =&gt; {`{`}{"\n"}
							{`      `}<span className="text-[#9cdcfe]">c</span>.<span className="text-[#9cdcfe]">state</span>.<span className="text-[#9cdcfe]">count</span>++;{`\n`}
							{`      `}<span className="text-[#9cdcfe]">c</span>.<span className="text-[#dcdcaa]">broadcast</span>(<span className="text-[#ce9178]">"changed"</span>, <span className="text-[#9cdcfe]">c</span>.<span className="text-[#9cdcfe]">state</span>.<span className="text-[#9cdcfe]">count</span>);{`\n`}
							{`    `}{`}`}{`\n`}
							{`  `}{`}`}{`\n`}
							{`});`}
						</code>
					</pre>
				</div>
			</div>
		);
	}

	return (
		<Link
			ref={cardRef}
			href={href}
			className={`group relative block ${className}`}
			onMouseMove={handleMouseMove}
		>
			<div className="h-full border border-white/20 bg-white/[0.008] hover:border-white/30 hover:bg-white/[0.02] rounded-xl p-6 transition-colors relative overflow-hidden flex flex-col">
				<div className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-300 pointer-events-none bg-gradient-to-b from-white/[0.04] via-white/[0.01] to-transparent" />

				<div className="relative z-10 mb-6">
					<h3 className="text-lg font-semibold text-white mb-3">
						{title}
					</h3>
					<p className="text-white/40 text-sm leading-relaxed">
						{description}
					</p>
				</div>

				<div className="flex-1 flex items-center justify-center relative z-10">
					<div className="icon-spotlight-container w-16 h-16 flex items-center justify-center">
						<Icon icon={icon} className="w-12 h-12 text-white/40" />
					</div>
				</div>
			</div>
		</Link>
	);
}

export function FeaturesBentoBox() {
	const features = [
		{
			title: "Long-Lived Stateful Compute",
			description: "Like AWS Lambda but with persistent memory and no timeouts. Your actors remember state between requests and intelligently hibernate when idle to save resources.",
			href: "/docs/actors",
			icon: faBolt,
			variant: "large" as const,
		},
		{
			title: "Blazing-Fast Performance",
			description: "State stored on the same machine as compute. Ultra-fast reads/writes with no database round trips.",
			href: "/docs/actors/state",
			icon: faRocket,
			variant: "medium" as const,
		},
		{
			title: "Built-in Realtime",
			description: "WebSockets & SSE support out of the box. Update state and broadcast changes instantly.",
			href: "/docs/actors/events",
			icon: faBolt,
			variant: "medium" as const,
		},
		{
			title: "Fault Tolerant",
			description: "Built-in error handling & recovery",
			href: "/docs/actors/lifecycle",
			icon: faShieldHalved,
			variant: "small" as const,
		},
		{
			title: "Auto-Hibernation",
			description: "Actors sleep when idle, wake instantly on demand",
			href: "/docs/actors/lifecycle",
			icon: faMoon,
			variant: "small" as const,
		},
		{
			title: "Scheduling",
			description: "Persistent timeouts survive restarts and crashes",
			href: "/docs/actors/schedule",
			icon: faClock,
			variant: "small" as const,
		},
		{
			title: "RPC & Events",
			description: "Full-featured messaging system",
			href: "/docs/actors/actions",
			icon: faDatabase,
			variant: "small" as const,
		},
	];

	return (
		<section className="w-full">
			<div className="container relative mx-auto px-6 lg:px-16 xl:px-20 max-w-[1500px]">
				<h2 className="text-2xl sm:text-3xl font-700 text-white mb-8 text-center">
					Each lightweight Actor comes packed with features
				</h2>
				<div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-4 md:gap-4 xl:gap-6 auto-rows-[200px]">
					{/* Large hero feature */}
					<FeatureCard
						title={features[0].title}
						description={features[0].description}
						href={features[0].href}
						icon={features[0].icon}
						className="sm:col-span-2 md:col-span-2 sm:row-span-2 md:row-span-2"
						variant="large"
					/>

					{/* Medium features */}
					<FeatureCard
						title={features[1].title}
						description={features[1].description}
						href={features[1].href}
						icon={features[1].icon}
						className="sm:col-span-2 md:col-span-2"
						variant="medium"
					/>

					<FeatureCard
						title={features[2].title}
						description={features[2].description}
						href={features[2].href}
						icon={features[2].icon}
						className="sm:col-span-2 md:col-span-2"
						variant="medium"
					/>

					{/* Small features */}
					{features.slice(3).map((feature) => (
						<FeatureCard
							key={feature.href}
							title={feature.title}
							description={feature.description}
							href={feature.href}
							icon={feature.icon}
							className="sm:col-span-1 md:col-span-1"
							variant="small"
						/>
					))}
				</div>
			</div>
		</section>
	);
}
