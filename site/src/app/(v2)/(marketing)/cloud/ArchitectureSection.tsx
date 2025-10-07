"use client";

import {
	Icon,
	faArrowRight,
	faCloud,
	faDatabase,
	faServer,
	faLayerGroup,
} from "@rivet-gg/icons";
import Link from "next/link";

export const ArchitectureSection = () => {
	return (
		<div className="mx-auto max-w-7xl px-6 py-32 lg:py-40">
			<div className="text-center mb-16">
				<h2 className="text-4xl font-medium tracking-tight text-white">
					Connect to Your Applications
				</h2>
				<p className="mt-4 text-lg text-white/70">
					Rivet Cloud scales actors that connect seamlessly to your applications deployed anywhere
				</p>
			</div>

			{/* Architecture Diagram */}
			<div className="bg-[#0A0A0A] rounded-xl p-8 mb-16">
				<div className="grid grid-cols-1 lg:grid-cols-[1fr_auto_1fr] gap-8 items-center">
					{/* Your Applications */}
					<div className="text-center">
						<div className="bg-white/5 rounded-lg p-6 border border-white/10">
							<Icon icon={faServer} className="text-3xl text-white/90 mb-4" />
							<h3 className="text-xl font-medium text-white mb-2">Your Applications</h3>
							<p className="text-white/60 text-sm">
								Deploy on Railway, Vercel, AWS, GCP, or any platform
							</p>
						</div>
					</div>

					{/* Connection Arrows */}
					<div className="flex flex-row lg:flex-col items-center justify-center gap-4">
						<Icon icon={faArrowRight} className="text-xl text-white/40 rotate-90 lg:rotate-0" />
						<Icon icon={faArrowRight} className="text-xl text-white/40 -rotate-90 lg:rotate-180" />
					</div>

					{/* Rivet Engine */}
					<div className="text-center">
						<div className="bg-white/5 rounded-lg p-6 border border-white/10">
							<Icon icon={faLayerGroup} className="text-3xl text-white/90 mb-4" />
							<h3 className="text-xl font-medium text-white mb-2">Rivet Cloud</h3>
							<p className="text-white/60 text-sm">
								Actor scaling and storage with managed FoundationDB
							</p>
						</div>
					</div>
				</div>

			</div>

			{/* Platform Integration Details */}
			<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
				<div className="bg-white/2 rounded-lg p-6 border border-white/10">
					<div className="flex items-center gap-3 mb-4">
						<div className="w-8 h-8 bg-orange-500/20 rounded-lg flex items-center justify-center">
							<span className="text-orange-500 font-bold text-sm">R</span>
						</div>
						<h3 className="text-lg font-medium text-white">Railway</h3>
					</div>
					<p className="text-white/60 text-sm">
						Connect your Railway-deployed applications to Rivet actors for stateful, real-time functionality.
					</p>
				</div>

				<div className="bg-white/2 rounded-lg p-6 border border-white/10">
					<div className="flex items-center gap-3 mb-4">
						<div className="w-8 h-8 bg-black/20 rounded-lg flex items-center justify-center">
							<span className="text-black font-bold text-sm">V</span>
						</div>
						<h3 className="text-lg font-medium text-white">Vercel</h3>
					</div>
					<p className="text-white/60 text-sm">
						Integrate Vercel serverless functions with Rivet actors for persistent state and real-time features.
					</p>
				</div>

				<div className="bg-white/2 rounded-lg p-6 border border-white/10">
					<div className="flex items-center gap-3 mb-4">
						<Icon icon={faCloud} className="text-xl text-white/90" />
						<h3 className="text-lg font-medium text-white">Any Platform</h3>
					</div>
					<p className="text-white/60 text-sm">
						Connect applications from AWS, GCP, Azure, or any cloud platform to Rivet Cloud
					</p>
				</div>
			</div>

			{/* CTA */}
			<div className="text-center mt-12">
				<Link
					href="/docs/self-hosting/connect-backend"
					className="inline-flex items-center gap-2 text-white/70 hover:text-white transition-colors"
				>
					<span>Learn how to connect your backend</span>
					<Icon icon={faArrowRight} />
				</Link>
			</div>
		</div>
	);
};
