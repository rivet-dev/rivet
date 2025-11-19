"use client";

import { Box, LayoutGrid, Terminal, Wrench } from "lucide-react";
import { motion } from "framer-motion";

export const IntegrationsSection = () => (
	<section className="py-32 bg-zinc-900/20 border-t border-white/5 relative overflow-hidden">
		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col md:flex-row items-center justify-between gap-12 mb-16">
				<motion.div
					initial={{ opacity: 0, x: -20 }}
					whileInView={{ opacity: 1, x: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="max-w-xl"
				>
					<h2 className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight">
						Stack Agnostic.
					</h2>
					<p className="text-lg text-zinc-400 leading-relaxed">
						Rivet actors are just standard TypeScript. They run in Docker, connect via WebSockets, and
						integrate effortlessly with your existing stack.
					</p>
				</motion.div>
			</div>

			<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
				{/* Category 1: Infrastructure (Blue) */}
				<div className="group p-6 rounded-2xl border border-white/5 bg-black/50 backdrop-blur-sm flex flex-col gap-4 relative overflow-hidden">
					{/* Top Shine Highlight - existing */}
					<div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent z-10" />

					{/* NEW: Top Left Reflection/Glow (Reduced opacity and soft fade) */}
					<div className="absolute inset-0 bg-[radial-gradient(circle_at_top_left,rgba(59,130,246,0.15)_0%,transparent_50%)] opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none" />
					{/* NEW: Sharp Edge Highlight (Masked to Fade - Fixed Clipping) */}
					<div className="absolute top-0 left-0 w-24 h-24 rounded-tl-2xl border-t border-l border-blue-500 opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none z-20 [mask-image:linear-gradient(135deg,black_0%,transparent_50%)]" />

					<div className="flex items-center gap-3 mb-2 relative z-10">
						{/* Updated Icon Container */}
						<div className="p-2 rounded bg-blue-500/10 text-blue-400 group-hover:bg-blue-500/20 group-hover:shadow-[0_0_15px_rgba(59,130,246,0.5)] transition-all duration-500">
							<Box className="w-4 h-4" />
						</div>
						<h4 className="text-sm font-medium text-white uppercase tracking-wider">Infrastructure</h4>
					</div>
					<div className="flex flex-wrap gap-2 relative z-10">
						{["Docker", "Kubernetes", "Fly.io", "Railway", "AWS ECS"].map((tech) => (
							<span
								key={tech}
								className="px-3 py-1.5 rounded-md bg-zinc-800/50 border border-white/5 text-xs text-zinc-300 hover:border-white/20 hover:text-white transition-colors cursor-default"
							>
								{tech}
							</span>
						))}
					</div>
				</div>

				{/* Category 2: Frameworks (Purple) */}
				<div className="group p-6 rounded-2xl border border-white/5 bg-black/50 backdrop-blur-sm flex flex-col gap-4 relative overflow-hidden">
					{/* Top Shine Highlight - existing */}
					<div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent z-10" />

					{/* NEW: Top Left Reflection/Glow (Reduced opacity and soft fade) */}
					<div className="absolute inset-0 bg-[radial-gradient(circle_at_top_left,rgba(168,85,247,0.15)_0%,transparent_50%)] opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none" />
					{/* NEW: Sharp Edge Highlight (Masked to Fade - Fixed Clipping) */}
					<div className="absolute top-0 left-0 w-24 h-24 rounded-tl-2xl border-t border-l border-purple-500 opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none z-20 [mask-image:linear-gradient(135deg,black_0%,transparent_50%)]" />

					<div className="flex items-center gap-3 mb-2 relative z-10">
						{/* Updated Icon Container */}
						<div className="p-2 rounded bg-purple-500/10 text-purple-400 group-hover:bg-purple-500/20 group-hover:shadow-[0_0_15px_rgba(168,85,247,0.5)] transition-all duration-500">
							<LayoutGrid className="w-4 h-4" />
						</div>
						<h4 className="text-sm font-medium text-white uppercase tracking-wider">Frameworks</h4>
					</div>
					<div className="flex flex-wrap gap-2 relative z-10">
						{["Next.js", "Remix", "React", "Vue", "Svelte", "Unity", "Godot"].map((tech) => (
							<span
								key={tech}
								className="px-3 py-1.5 rounded-md bg-zinc-800/50 border border-white/5 text-xs text-zinc-300 hover:border-white/20 hover:text-white transition-colors cursor-default"
							>
								{tech}
							</span>
						))}
					</div>
				</div>

				{/* Category 3: Runtimes (Yellow) */}
				<div className="group p-6 rounded-2xl border border-white/5 bg-black/50 backdrop-blur-sm flex flex-col gap-4 relative overflow-hidden">
					{/* Top Shine Highlight - existing */}
					<div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent z-10" />

					{/* NEW: Top Left Reflection/Glow (Reduced opacity and soft fade) */}
					<div className="absolute inset-0 bg-[radial-gradient(circle_at_top_left,rgba(234,179,8,0.15)_0%,transparent_50%)] opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none" />
					{/* NEW: Sharp Edge Highlight (Masked to Fade - Fixed Clipping) */}
					<div className="absolute top-0 left-0 w-24 h-24 rounded-tl-2xl border-t border-l border-yellow-500 opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none z-20 [mask-image:linear-gradient(135deg,black_0%,transparent_50%)]" />

					<div className="flex items-center gap-3 mb-2 relative z-10">
						{/* Updated Icon Container */}
						<div className="p-2 rounded bg-yellow-500/10 text-yellow-400 group-hover:bg-yellow-500/20 group-hover:shadow-[0_0_15px_rgba(234,179,8,0.5)] transition-all duration-500">
							<Terminal className="w-4 h-4" />
						</div>
						<h4 className="text-sm font-medium text-white uppercase tracking-wider">Runtimes</h4>
					</div>
					<div className="flex flex-wrap gap-2 relative z-10">
						{["Node.js", "Bun", "Deno", "Cloudflare Workers"].map((tech) => (
							<span
								key={tech}
								className="px-3 py-1.5 rounded-md bg-zinc-800/50 border border-white/5 text-xs text-zinc-300 hover:border-white/20 hover:text-white transition-colors cursor-default"
							>
								{tech}
							</span>
						))}
					</div>
				</div>

				{/* Category 4: Tools (Emerald) */}
				<div className="group p-6 rounded-2xl border border-white/5 bg-black/50 backdrop-blur-sm flex flex-col gap-4 relative overflow-hidden">
					{/* Top Shine Highlight - existing */}
					<div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent z-10" />

					{/* NEW: Top Left Reflection/Glow (Reduced opacity and soft fade) */}
					<div className="absolute inset-0 bg-[radial-gradient(circle_at_top_left,rgba(16,185,129,0.15)_0%,transparent_50%)] opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none" />
					{/* NEW: Sharp Edge Highlight (Masked to Fade - Fixed Clipping) */}
					<div className="absolute top-0 left-0 w-24 h-24 rounded-tl-2xl border-t border-l border-emerald-500 opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none z-20 [mask-image:linear-gradient(135deg,black_0%,transparent_50%)]" />

					<div className="flex items-center gap-3 mb-2 relative z-10">
						{/* Updated Icon Container */}
						<div className="p-2 rounded bg-emerald-500/10 text-emerald-400 group-hover:bg-emerald-500/20 group-hover:shadow-[0_0_15px_rgba(16,185,129,0.5)] transition-all duration-500">
							<Wrench className="w-4 h-4" />
						</div>
						<h4 className="text-sm font-medium text-white uppercase tracking-wider">Tools</h4>
					</div>
					<div className="flex flex-wrap gap-2 relative z-10">
						{["TypeScript", "ESLint", "Prettier", "Vite", "Turborepo"].map((tech) => (
							<span
								key={tech}
								className="px-3 py-1.5 rounded-md bg-zinc-800/50 border border-white/5 text-xs text-zinc-300 hover:border-white/20 hover:text-white transition-colors cursor-default"
							>
								{tech}
							</span>
						))}
					</div>
				</div>
			</div>
		</div>
	</section>
);

