"use client";

import { Github, Cloud, Server, Check } from "lucide-react";
import { motion } from "framer-motion";

export const HostingSection = () => (
	<section className="py-24 bg-black border-t border-white/10">
		<div className="max-w-7xl mx-auto px-6">
			<div className="text-center mb-16">
				<motion.h2
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="text-3xl md:text-4xl font-medium text-white mb-4 tracking-tight"
				>
					Deploy your way.
				</motion.h2>
				<motion.p
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.1 }}
					className="text-zinc-400 max-w-2xl mx-auto"
				>
					Start with the open-source binary on your laptop. Scale with Rivet Cloud. Go hybrid when you need
					total control over data residency.
				</motion.p>
			</div>

			<div className="grid grid-cols-1 md:grid-cols-3 gap-8">
				{/* Card 1: Open Source */}
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.2 }}
					className="p-8 rounded-2xl border border-white/10 bg-white/[0.02] hover:bg-white/[0.05] backdrop-blur-sm transition-all duration-300 flex flex-col hover:border-white/20 hover:shadow-[0_0_30px_-10px_rgba(255,255,255,0.1)]"
				>
					<div className="w-12 h-12 rounded-lg bg-white/5 flex items-center justify-center mb-6 text-white">
						<Github className="w-6 h-6" />
					</div>
					<h3 className="text-xl font-medium text-white mb-3">Open Source</h3>
					<p className="text-zinc-400 text-sm leading-relaxed mb-6 flex-grow">
						The core engine is 100% open source and compiles to a single binary. Run it locally, on a VPS,
						or inside Kubernetes. No vendor lock-in.
					</p>
					<div className="bg-black rounded-lg border border-white/10 p-4 font-mono text-xs text-zinc-300">
						<div className="flex gap-2">
							<span className="text-[#FF4500] select-none">$</span>
							<span>curl -fsSL https://rivet.gg/install.sh | sh</span>
						</div>
						<div className="flex gap-2 mt-2">
							<span className="text-[#FF4500] select-none">$</span>
							<span>rivet-server start</span>
						</div>
					</div>
				</motion.div>

				{/* Card 2: Rivet Cloud */}
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.3 }}
					className="p-8 rounded-2xl border border-white/10 bg-gradient-to-b from-[#FF4500]/10 to-transparent relative overflow-hidden group flex flex-col backdrop-blur-sm hover:border-[#FF4500]/30 transition-colors"
				>
					<div className="w-12 h-12 rounded-lg bg-[#FF4500]/10 flex items-center justify-center mb-6 text-[#FF4500] relative z-10">
						<Cloud className="w-6 h-6" />
					</div>
					<h3 className="text-xl font-medium text-white mb-3 relative z-10">Rivet Cloud</h3>
					<p className="text-zinc-400 text-sm leading-relaxed mb-6 relative z-10 flex-grow">
						The fully managed control plane. We handle the orchestration, monitoring, and edge routing. You
						just connect your compute and go.
					</p>
					<ul className="space-y-2 relative z-10">
						{["Managed Orchestration", "Global Edge Network", "Instant Scaling"].map((item) => (
							<li key={item} className="flex items-center gap-2 text-xs text-zinc-300">
								<Check className="w-3 h-3 text-[#FF4500]" /> {item}
							</li>
						))}
					</ul>
				</motion.div>

				{/* Card 3: Hybrid */}
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.4 }}
					className="p-8 rounded-2xl border border-white/10 bg-white/[0.02] hover:bg-white/[0.05] backdrop-blur-sm transition-all duration-300 flex flex-col hover:border-white/20 hover:shadow-[0_0_30px_-10px_rgba(255,255,255,0.1)]"
				>
					<div className="w-12 h-12 rounded-lg bg-white/5 flex items-center justify-center mb-6 text-white">
						<Server className="w-6 h-6" />
					</div>
					<h3 className="text-xl font-medium text-white mb-3">Hybrid & On-Prem</h3>
					<p className="text-zinc-400 text-sm leading-relaxed mb-6 flex-grow">
						Keep sensitive actors on your own private VPC for compliance, while offloading public traffic
						to Rivet Cloud. Managed via a single dashboard.
					</p>
					<div className="h-24 rounded-lg border border-white/5 bg-black/50 flex items-center justify-center gap-4 px-4">
						<div className="text-xs text-zinc-500 text-center">
							<div className="mb-1">
								<Server className="w-4 h-4 mx-auto" />
							</div>
							Private
						</div>
						<div className="h-px flex-1 bg-gradient-to-r from-zinc-800 via-[#FF4500]/50 to-zinc-800" />
						<div className="text-xs text-zinc-500 text-center">
							<div className="mb-1">
								<Cloud className="w-4 h-4 mx-auto" />
							</div>
							Cloud
						</div>
					</div>
				</motion.div>
			</div>
		</div>
	</section>
);

