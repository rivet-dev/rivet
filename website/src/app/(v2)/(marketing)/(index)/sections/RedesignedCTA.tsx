"use client";

import { motion } from "framer-motion";

export const RedesignedCTA = () => (
	<section className="py-32 text-center px-6 border-t border-white/10 relative overflow-hidden">
		<div className="absolute inset-0 bg-gradient-to-b from-black to-zinc-900/50 z-0" />
		<motion.div
			animate={{ opacity: [0.3, 0.5, 0.3] }}
			transition={{ duration: 4, repeat: Infinity }}
			className="absolute inset-0 bg-[radial-gradient(ellipse_at_center,_var(--tw-gradient-stops))] from-[#FF4500]/10 via-transparent to-transparent opacity-50 pointer-events-none"
		/>
		<div className="max-w-3xl mx-auto relative z-10">
			<motion.h2
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5 }}
				className="text-4xl md:text-5xl font-medium text-white mb-8 tracking-tight"
			>
				Ready to build better backends?
			</motion.h2>
			<motion.p
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5, delay: 0.1 }}
				className="text-lg text-zinc-400 mb-10 leading-relaxed"
			>
				Join thousands of developers building the next generation of{" "}
				<br className="hidden md:block" />
				realtime, stateful applications with Rivet.
			</motion.p>
			<motion.div
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5, delay: 0.2 }}
				className="flex flex-col sm:flex-row items-center justify-center gap-4"
			>
				<button className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white shadow-sm hover:border-white/20 transition-colors">
					Get Started for Free
				</button>
				<button className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white shadow-sm hover:border-white/20 transition-colors">
					Read the Docs
				</button>
			</motion.div>
		</div>
	</section>
);

