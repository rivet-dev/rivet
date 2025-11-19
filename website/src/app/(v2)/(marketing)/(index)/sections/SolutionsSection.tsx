"use client";

import {
	Bot,
	FileText,
	Workflow,
	RefreshCw,
	MessageSquare,
	Database,
	Gamepad2,
	Clock,
	Gauge,
	ArrowRight,
} from "lucide-react";
import { motion } from "framer-motion";

export const SolutionsSection = () => {
	const solutions = [
		{
			title: "AI Agent",
			description: "Build durable AI assistants with persistent memory and realtime streaming",
			icon: <Bot className="w-5 h-5" />,
		},
		{
			title: "Collaborative State",
			description: "Collaborative documents with CRDTs and realtime synchronization",
			icon: <FileText className="w-5 h-5" />,
		},
		{
			title: "Workflows",
			description: "Durable multi-step workflows with automatic state management",
			icon: <Workflow className="w-5 h-5" />,
		},
		{
			title: "Local-First Sync",
			description: "Offline-first applications with server synchronization",
			icon: <RefreshCw className="w-5 h-5" />,
		},
		{
			title: "Bots",
			description: "Discord, Slack, or autonomous bots with persistent state",
			icon: <MessageSquare className="w-5 h-5" />,
		},
		{
			title: "User Session Store",
			description: "Isolated data stores for each user with zero-latency access",
			icon: <Database className="w-5 h-5" />,
		},
		{
			title: "Multiplayer Game",
			description: "Authoritative game servers with realtime state synchronization",
			icon: <Gamepad2 className="w-5 h-5" />,
		},
		{
			title: "Background Jobs",
			description: "Scheduled and recurring jobs without external queue infrastructure",
			icon: <Clock className="w-5 h-5" />,
		},
		{
			title: "Rate Limiting",
			description: "Distributed rate limiting with in-memory counters",
			icon: <Gauge className="w-5 h-5" />,
		},
	];

	return (
		<section id="solutions" className="py-32 bg-black relative border-t border-white/10">
			<div className="max-w-7xl mx-auto px-6">
				<div className="text-center mb-20">
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						Build anything stateful.
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg text-zinc-400 max-w-2xl mx-auto leading-relaxed"
					>
						If it needs to remember something, it belongs in an Actor.
					</motion.p>
				</div>

				<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
					{solutions.map((solution, idx) => (
						<motion.div
							key={idx}
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: idx * 0.05 }}
							className="p-6 rounded-xl border border-white/10 bg-white/[0.02] hover:bg-white/[0.05] backdrop-blur-sm transition-all duration-300 group flex flex-col justify-between hover:border-white/20 hover:shadow-[0_0_30px_-10px_rgba(255,255,255,0.1)] relative overflow-hidden"
						>
							{/* Top Shine Highlight */}
							<div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent opacity-50 group-hover:opacity-100 transition-opacity z-10" />
							<div className="flex items-center justify-between mb-4 relative z-10">
								<div className="flex items-center gap-3">
									<div className="text-white/80">{solution.icon}</div>
									<h3 className="font-medium text-white tracking-tight">{solution.title}</h3>
								</div>
								<ArrowRight className="w-4 h-4 text-zinc-600 group-hover:text-white transition-colors" />
							</div>
							<p className="text-sm text-zinc-400 leading-relaxed relative z-10">{solution.description}</p>
						</motion.div>
					))}
				</div>
			</div>
		</section>
	);
};

