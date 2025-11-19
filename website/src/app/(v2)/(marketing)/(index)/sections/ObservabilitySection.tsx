"use client";

import { Eye, Activity, Terminal, Wifi, ArrowRight, Play } from "lucide-react";
import { motion } from "framer-motion";

export const ObservabilitySection = () => {
	const features = [
		{
			title: "Live State Inspection",
			description:
				"View and edit your actor state in real-time as messages are sent and processed",
			icon: <Eye className="w-5 h-5 text-emerald-400" />,
		},
		{
			title: "Event Monitoring",
			description:
				"See all events happening in your actor in real-time - track every state change and action as it happens",
			icon: <Activity className="w-5 h-5 text-blue-400" />,
		},
		{
			title: "REPL",
			description:
				"Debug your actor in real-time - call actions, subscribe to events, and interact directly with your code",
			icon: <Terminal className="w-5 h-5 text-[#FF4500]" />,
		},
		{
			title: "Connection Inspection",
			description: "Monitor active connections with state and parameters for each client",
			icon: <Wifi className="w-5 h-5 text-purple-400" />,
		},
	];

	return (
		<section className="py-32 bg-black border-t border-white/10">
			<div className="max-w-7xl mx-auto px-6">
				<div className="mb-20">
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						Built-In Observability
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg text-zinc-400 max-w-2xl mb-8"
					>
						Powerful debugging and monitoring tools that work seamlessly from local development to production
						at scale.
					</motion.p>
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="flex flex-col sm:flex-row gap-4"
					>
						<button className="inline-flex items-center gap-2 text-sm font-medium text-white bg-zinc-800 px-4 py-2 rounded-full hover:bg-zinc-700 transition-colors border border-white/10">
							Visit The Inspector
							<ArrowRight className="w-4 h-4" />
						</button>
						<button className="inline-flex items-center gap-2 text-sm font-medium text-zinc-400 hover:text-white transition-colors px-4 py-2">
							<Play className="w-4 h-4" />
							Watch Demo
						</button>
					</motion.div>
				</div>

				<div className="grid lg:grid-cols-2 gap-16 items-center">
					{/* Feature List */}
					<div className="grid gap-10">
						{features.map((feat, idx) => (
							<motion.div
								key={idx}
								initial={{ opacity: 0, x: -20 }}
								whileInView={{ opacity: 1, x: 0 }}
								viewport={{ once: true }}
								transition={{ duration: 0.5, delay: idx * 0.1 }}
								className="flex gap-4"
							>
								<div className="flex-shrink-0 w-10 h-10 rounded-lg bg-zinc-900 border border-white/10 flex items-center justify-center">
									{feat.icon}
								</div>
								<div>
									<h3 className="text-lg font-medium text-white mb-2">{feat.title}</h3>
									<p className="text-sm text-zinc-400 leading-relaxed">{feat.description}</p>
								</div>
							</motion.div>
						))}
					</div>

					{/* Empty Window Frame */}
					<motion.div
						initial={{ opacity: 0, scale: 0.95 }}
						whileInView={{ opacity: 1, scale: 1 }}
						viewport={{ once: true }}
						transition={{ duration: 0.7 }}
						className="relative"
					>
						<div className="absolute -inset-4 bg-gradient-to-r from-[#FF4500]/20 to-blue-500/20 rounded-3xl blur-2xl opacity-20" />
						<div className="relative rounded-xl border border-white/10 bg-zinc-900/50 backdrop-blur-xl shadow-2xl overflow-hidden aspect-video flex flex-col">
							{/* Window Bar */}
							<div className="h-10 border-b border-white/5 bg-white/5 flex items-center px-4 gap-2 flex-shrink-0">
								<div className="w-3 h-3 rounded-full bg-[#FF5F57] border border-[#E0443E]" />
								<div className="w-3 h-3 rounded-full bg-[#FEBC2E] border border-[#D89E24]" />
								<div className="w-3 h-3 rounded-full bg-[#28C840] border border-[#1AAB29]" />
							</div>
							{/* Content Area - Placeholder for Image */}
							<div className="flex-grow flex items-center justify-center bg-zinc-900/50">
								<div className="text-zinc-600 text-sm font-mono">Dashboard Placeholder</div>
							</div>
						</div>
					</motion.div>
				</div>
			</div>
		</section>
	);
};

