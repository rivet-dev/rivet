import { Icon, faArrowRight } from "@rivet-gg/icons";

function DocsLink({ href }: { href: string }) {
	return (
		<a
			href={href}
			className="inline-flex items-center gap-1 text-sm text-white/40 hover:text-white transition-colors group/link"
		>
			Documentation
			<Icon
				icon={faArrowRight}
				className="w-3 h-3 transition-transform group-hover/link:translate-x-0.5"
			/>
		</a>
	);
}

interface FeatureCardProps {
	title: string;
	description: string;
	color: string;
	className?: string;
	docLink?: string;
}

function FeatureCard({
	title,
	description,
	color,
	className = "",
	docLink,
}: FeatureCardProps) {
	return (
		<div className={`group relative ${className}`}>
			<div className="h-full border border-white/10 rounded-xl p-6 overflow-hidden">
				<div className="relative z-10">
					<h3 className="text-lg font-semibold text-white mb-3">
						{title}
					</h3>
					<p className="text-white/40 text-sm leading-relaxed mb-3">
						{description}
					</p>
					{docLink && <DocsLink href={docLink} />}
				</div>
				<div
					className={`absolute bottom-4 left-6 w-8 h-1 rounded-full ${
						color === "orange-500"
							? "bg-orange-500"
							: color === "blue-500"
								? "bg-blue-500"
								: color === "green-500"
									? "bg-green-500"
									: color === "purple-500"
										? "bg-purple-500"
										: color === "pink-500"
											? "bg-pink-500"
											: color === "teal-500"
												? "bg-teal-500"
												: color === "amber-500"
													? "bg-amber-500"
													: "bg-gray-500"
					}`}
				></div>
			</div>
		</div>
	);
}

interface LargeFeatureCardProps {
	title: string;
	description: string;
	color: string;
	className?: string;
	docLink?: string;
}

function LargeFeatureCard({
	title,
	description,
	color,
	className = "",
	docLink,
}: LargeFeatureCardProps) {
	return (
		<div className={`group relative ${className}`}>
			<div className="relative h-full border border-white/10 rounded-xl p-6 overflow-hidden">
				<div className="relative z-10">
					<h3 className="text-lg font-semibold text-white mb-3">
						{title}
					</h3>
					<p className="text-white/40 text-sm leading-relaxed mb-3">
						{description}
					</p>
					{docLink && <DocsLink href={docLink} />}
				</div>
				<div
					className={`absolute bottom-4 left-6 w-8 h-1 rounded-full ${
						color === "orange-500"
							? "bg-orange-500"
							: color === "blue-500"
								? "bg-blue-500"
								: color === "green-500"
									? "bg-green-500"
									: color === "purple-500"
										? "bg-purple-500"
										: color === "pink-500"
											? "bg-pink-500"
											: color === "teal-500"
												? "bg-teal-500"
												: color === "amber-500"
													? "bg-amber-500"
													: "bg-gray-500"
					}`}
				></div>
			</div>
		</div>
	);
}

export function FeaturesSection() {
	return (
		<section className="w-full py-24">
			<div className="mx-auto max-w-7xl">
				<div className="text-center mb-16">
					<h2 className="text-4xl sm:text-5xl font-700 text-white mb-6">
						Built for Modern Applications
					</h2>
					<p className="text-lg sm:text-xl font-500 text-white/60 max-w-2xl mx-auto">
						Everything you need to build fast, scalable, and
						real-time applications without the complexity.
					</p>
				</div>

				<div className="grid grid-cols-4 grid-rows-2 gap-4 h-[500px]">
					<LargeFeatureCard
						className="col-span-2 row-span-1"
						title="Long-Lived, Stateful Compute"
						description="Each unit of compute is like a tiny server that remembers things between requests – no need to re-fetch data from a database or worry about timeouts. Like AWS Lambda, but with memory and no timeouts."
						color="orange-500"
						docLink="/docs/actors"
					/>

					<FeatureCard
						className="col-span-1 row-span-1"
						title="Blazing-Fast Reads & Writes"
						description="State is stored on the same machine as your compute, so reads and writes are ultra-fast. No database round trips, no latency spikes."
						color="blue-500"
						docLink="/docs/actors/state"
					/>

					<FeatureCard
						className="col-span-1 row-span-1"
						title="Realtime, Made Simple"
						description="Update state and broadcast changes in realtime with WebSockets or SSE. No external pub/sub systems, no polling – just built-in low-latency events."
						color="green-500"
						docLink="/docs/actors/events"
					/>

					<FeatureCard
						className="col-span-1 row-span-1"
						title="Store Data Near Your Users"
						description="Your state lives close to your users on the edge – not in a faraway data center – so every interaction feels instant. (Not all platforms supported.)"
						color="purple-500"
						docLink="/docs/general/edge"
					/>

					<FeatureCard
						className="col-span-1 row-span-1"
						title="Infinitely Scalable"
						description="Automatically scale from zero to millions of concurrent actors. Pay only for what you use with instant scaling and no cold starts."
						color="pink-500"
						docLink="/docs/actors/scaling"
					/>

					<FeatureCard
						className="col-span-1 row-span-1"
						title="Fault Tolerant"
						description="Built-in error handling and recovery. Actors automatically restart on failure while preserving state integrity and continuing operations."
						color="teal-500"
						docLink="/docs/cloud/durability"
					/>

					<FeatureCard
						className="col-span-1 row-span-1"
						title="Type Safety"
						description="End-to-end TypeScript safety between clients and actors with full type inference and compile-time checking."
						color="amber-500"
						docLink="/docs/actors/helper-types"
					/>
				</div>
			</div>
		</section>
	);
}
