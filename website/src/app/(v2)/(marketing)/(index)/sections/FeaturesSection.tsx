"use client";

import { useRef } from "react";
import { IconWithSpotlight } from "./IconWithSpotlight";

interface FeatureCardProps {
	title: string;
	description: string;
	className?: string;
	docLink?: string;
	iconPath?: string;
}

function FeatureCard({
	title,
	description,
	className = "",
	docLink,
	iconPath,
}: FeatureCardProps) {
	const cardRef = useRef<HTMLAnchorElement>(null);

	const handleMouseMove = (e: React.MouseEvent<HTMLAnchorElement>) => {
		if (!cardRef.current) return;
		const card = cardRef.current;

		// Find the icon container and convert coordinates to icon-relative
		const iconContainer = card.querySelector('.icon-spotlight-container') as HTMLElement;
		if (!iconContainer) return;

		// Get the icon's position relative to viewport
		const iconRect = iconContainer.getBoundingClientRect();

		// Calculate mouse position relative to the icon (not the card)
		const x = ((e.clientX - iconRect.left) / iconRect.width) * 100;
		const y = ((e.clientY - iconRect.top) / iconRect.height) * 100;

		// Set CSS custom properties on the icon container
		iconContainer.style.setProperty('--mouse-x', `${x}%`);
		iconContainer.style.setProperty('--mouse-y', `${y}%`);
	};

	return (
		<a
			ref={cardRef}
			href={docLink}
			className={`group relative block ${className}`}
			onMouseMove={handleMouseMove}
		>
			<div className="h-full border border-white/10 hover:border-white/30 hover:bg-white/[0.02] rounded-xl p-6 transition-colors relative overflow-hidden">
				{iconPath && (
					<>
						<div className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-300 pointer-events-none bg-gradient-to-b from-white/[0.04] via-white/[0.01] to-transparent" />
						<div className="mb-6 flex justify-center relative z-10">
							<IconWithSpotlight iconPath={iconPath} title={title} />
						</div>
					</>
				)}
				<h3 className="text-lg font-semibold text-white mb-3 relative z-10">{title}</h3>
				<p className="text-white/40 text-sm leading-relaxed relative z-10">
					{description}
				</p>
			</div>
		</a>
	);
}

export function FeaturesSection() {
	return (
		<section className="w-full py-24">
			<div className="mx-auto max-w-7xl">
				<div className="text-center mb-16">
					<h2 className="text-2xl sm:text-3xl font-700 text-white mb-6">
						Built for Modern Applications
					</h2>
					<p className="text-lg sm:text-xl font-500 text-white/60 max-w-2xl mx-auto">
						Everything you need to build fast, scalable, and realtime
						applications without the complexity.
					</p>
				</div>

				<div className="grid grid-cols-1 md:grid-cols-3 gap-4">
					<FeatureCard
						title="Long-Lived, Stateful Compute"
						description="Each unit of compute is like a tiny server that remembers things between requests – no need to re-fetch data from a database or worry about timeouts. Like AWS Lambda, but with memory and no timeouts."
						docLink="/docs/actors"
						iconPath="/icons/microchip.svg"
					/>

					<FeatureCard
						title="No Database Round Trips"
						description="State is stored on the same machine as your compute, so reads and writes are ultra-fast. No database round trips, no latency spikes."
						docLink="/docs/actors/state"
						iconPath="/icons/database.svg"
					/>

					<FeatureCard
						title="Realtime, Made Simple"
						description="Update state and broadcast changes in realtime with WebSockets or SSE. No external pub/sub systems, no polling – just built-in low-latency events."
						docLink="/docs/actors/events"
						iconPath="/icons/bolt.svg"
					/>

					<FeatureCard
						title="Sleep When Idle, No Cold Starts"
						description="Actors automatically hibernate when idle and wake up instantly on demand with zero cold start delays. Only pay for active compute time while keeping your state ready."
						docLink="/docs/actors/lifecycle"
					/>

					<FeatureCard
						title="Architected For Insane Scale"
						description="Automatically scale from zero to millions of concurrent actors. Pay only for what you use with instant scaling and no cold starts."
						docLink="/docs/actors/scaling"
					/>

					<FeatureCard
						title="Resilient by Design"
						description="Built-in failover and recovery. Actors automatically restart on failure while preserving state integrity and continuing operations."
						docLink="/docs/actors/lifecycle"
					/>

					<FeatureCard
						title="Store State at the Edge"
						description="Your state lives close to your users on the edge – not in a faraway data center – so every interaction feels instant."
						docLink="/docs/general/edge"
					/>

					<FeatureCard
						title="Built on Web Standards"
						description="Powered by WebSockets, SSE, and HTTP. Works with existing libraries, tools, and web browsers. Drop down to raw fetch handlers when you need full control."
						docLink="/docs/actors/fetch-and-websocket-handler/"
					/>

					<FeatureCard
						title="No Vendor Lock-In"
						description="Open-source and fully self-hostable. Works with Node.js, Bun, Deno, and Cloudflare. Run on any cloud provider or on-premises infrastructure."
						docLink="/docs/self-hosting"
					/>
				</div>
			</div>
		</section>
	);
}
