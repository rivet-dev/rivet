"use client";

import { motion } from "framer-motion";

const StatItem = ({ value, label }) => (
	<div className="flex flex-col items-start pl-6 border-l border-white/10 group hover:border-[#FF4500]/50 transition-colors duration-500">
		<span className="text-3xl font-medium text-white tracking-tighter mb-1 group-hover:text-[#FF4500] transition-colors">
			{value}
		</span>
		<span className="text-xs text-zinc-500 font-medium tracking-widest uppercase">{label}</span>
	</div>
);

export const StatsSection = () => (
	<section className="border-y border-white/5 bg-white/[0.02] backdrop-blur-sm">
		<div className="max-w-7xl mx-auto px-6 py-16">
			<motion.div
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5, staggerChildren: 0.1 }}
				className="grid grid-cols-2 md:grid-cols-4 gap-8 md:gap-0"
			>
				<StatItem value="< 1ms" label="Read/Write Latency" />
				<StatItem value="∞" label="Horizontal Scale" />
				<StatItem value="100%" label="Open Source" />
				<StatItem value="Zero" label="Database Config" />
			</motion.div>
		</div>
	</section>
);
