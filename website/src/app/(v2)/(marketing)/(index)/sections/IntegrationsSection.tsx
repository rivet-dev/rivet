"use client";

import { Box, LayoutGrid, Terminal, Wrench } from "lucide-react";
import { motion } from "framer-motion";

export const IntegrationsSection = () => (
	<section className="py-24 bg-zinc-900/20 border-t border-white/5 relative overflow-hidden">
		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col md:flex-row items-center justify-between gap-12 mb-16">
				<motion.div
					initial={{ opacity: 0, x: -20 }}
					whileInView={{ opacity: 1, x: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="max-w-xl"
				>
					<h2 className="text-3xl md:text-4xl font-medium text-white mb-4 tracking-tight">
						Stack Agnostic.
					</h2>
					<p className="text-zinc-400 text-lg leading-relaxed">
						Rivet actors are just standard TypeScript. They run in Docker, connect via WebSockets, and
						integrate effortlessly with your existing stack.
					</p>
				</motion.div>
			</div>

			<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
				{/* Category 1: Infrastructure */}
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.2 }}
					className="p-6 rounded-2xl border border-white/5 bg-white/[0.02] backdrop-blur-md flex flex-col gap-4 hover:bg-white/[0.04] transition-colors hover:border-white/10 hover:shadow-lg"
				>
					<div className="flex items-center gap-3 mb-2">
						<div className="p-2 rounded bg-blue-500/10 text-blue-400">
							<Box className="w-4 h-4" />
						</div>
						<h4 className="text-sm font-medium text-white uppercase tracking-wider">Infrastructure</h4>
					</div>
					<div className="flex flex-wrap gap-2">
						{["Docker", "Kubernetes", "Fly.io", "Railway", "AWS ECS"].map((tech) => (
							<span
								key={tech}
								className="px-3 py-1.5 rounded-md bg-zinc-800/50 border border-white/5 text-xs text-zinc-300 hover:border-white/20 hover:text-white transition-colors cursor-default"
							>
								{tech}
							</span>
						))}
					</div>
				</motion.div>

				{/* Category 2: Frameworks */}
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.3 }}
					className="p-6 rounded-2xl border border-white/5 bg-white/[0.02] backdrop-blur-md flex flex-col gap-4 hover:bg-white/[0.04] transition-colors hover:border-white/10 hover:shadow-lg"
				>
					<div className="flex items-center gap-3 mb-2">
						<div className="p-2 rounded bg-purple-500/10 text-purple-400">
							<LayoutGrid className="w-4 h-4" />
						</div>
						<h4 className="text-sm font-medium text-white uppercase tracking-wider">Frameworks</h4>
					</div>
					<div className="flex flex-wrap gap-2">
						{["Next.js", "Remix", "React", "Vue", "Svelte", "Unity", "Godot"].map((tech) => (
							<span
								key={tech}
								className="px-3 py-1.5 rounded-md bg-zinc-800/50 border border-white/5 text-xs text-zinc-300 hover:border-white/20 hover:text-white transition-colors cursor-default"
							>
								{tech}
							</span>
						))}
					</div>
				</motion.div>

				{/* Category 3: Runtimes */}
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.4 }}
					className="p-6 rounded-2xl border border-white/5 bg-white/[0.02] backdrop-blur-md flex flex-col gap-4 hover:bg-white/[0.04] transition-colors hover:border-white/10 hover:shadow-lg"
				>
					<div className="flex items-center gap-3 mb-2">
						<div className="p-2 rounded bg-yellow-500/10 text-yellow-400">
							<Terminal className="w-4 h-4" />
						</div>
						<h4 className="text-sm font-medium text-white uppercase tracking-wider">Runtimes</h4>
					</div>
					<div className="flex flex-wrap gap-2">
						{["Node.js", "Bun", "Deno", "Cloudflare Workers"].map((tech) => (
							<span
								key={tech}
								className="px-3 py-1.5 rounded-md bg-zinc-800/50 border border-white/5 text-xs text-zinc-300 hover:border-white/20 hover:text-white transition-colors cursor-default"
							>
								{tech}
							</span>
						))}
					</div>
				</motion.div>

				{/* Category 4: Tools */}
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.5 }}
					className="p-6 rounded-2xl border border-white/5 bg-white/[0.02] backdrop-blur-md flex flex-col gap-4 hover:bg-white/[0.04] transition-colors hover:border-white/10 hover:shadow-lg"
				>
					<div className="flex items-center gap-3 mb-2">
						<div className="p-2 rounded bg-emerald-500/10 text-emerald-400">
							<Wrench className="w-4 h-4" />
						</div>
						<h4 className="text-sm font-medium text-white uppercase tracking-wider">Tools</h4>
					</div>
					<div className="flex flex-wrap gap-2">
						{["TypeScript", "ESLint", "Prettier", "Vite", "Turborepo"].map((tech) => (
							<span
								key={tech}
								className="px-3 py-1.5 rounded-md bg-zinc-800/50 border border-white/5 text-xs text-zinc-300 hover:border-white/20 hover:text-white transition-colors cursor-default"
							>
								{tech}
							</span>
						))}
					</div>
				</motion.div>
			</div>
		</div>
	</section>
);

