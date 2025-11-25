"use client";

import { useEffect, useState } from "react";
import { Server, Box, Database, Cpu, Check } from "lucide-react";
import { motion } from "framer-motion";

const ArchitectureComparison = () => {
	const [activeStep, setActiveStep] = useState(0);

	useEffect(() => {
		const interval = setInterval(() => {
			setActiveStep((prev) => (prev + 1) % 4);
		}, 2000);
		return () => clearInterval(interval);
	}, []);

	return (
		<div className="grid grid-cols-1 md:grid-cols-2 gap-8 w-full">
			{/* Traditional Serverless */}
			<motion.div
				initial={{ opacity: 0, x: -20 }}
				whileInView={{ opacity: 1, x: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5 }}
				className="p-6 rounded-2xl border border-white/5 bg-zinc-900/30 backdrop-blur-sm hover:bg-zinc-900/50 transition-colors duration-500 relative overflow-hidden"
			>
				{/* Top Shine Highlight */}
				<div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent z-10" />
				<div className="flex items-center gap-2 mb-6 relative z-10">
					<div className="p-2 rounded bg-red-500/10 text-red-400">
						<Server className="w-4 h-4" />
					</div>
					<h4 className="font-semibold text-white">Traditional Serverless</h4>
				</div>

				<div className="relative h-48 flex flex-col items-center justify-center gap-8">
					<div className="flex items-center gap-8 w-full justify-center">
						<div
							className={`px-4 py-2 rounded border ${
								activeStep === 0
									? "border-white bg-white text-black"
									: "border-zinc-700 text-zinc-500"
							} transition-colors text-xs font-mono`}
						>
							Client
						</div>
						<div
							className={`h-[1px] w-12 ${
								activeStep === 0 ? "bg-white" : "bg-zinc-800"
							} transition-colors relative`}
						>
							<div
								className={`absolute -top-1 right-0 w-2 h-2 border-t border-r ${
									activeStep === 0 ? "border-white" : "border-zinc-800"
								} rotate-45`}
							/>
						</div>
						<div
							className={`px-4 py-2 rounded border ${
								activeStep === 1 || activeStep === 3
									? "border-blue-500 text-blue-400 bg-blue-500/10"
									: "border-zinc-700 text-zinc-500"
							} transition-colors text-xs font-mono`}
						>
							Lambda
						</div>
					</div>

					<div className="flex flex-col items-center gap-2">
						<div
							className={`w-[1px] h-8 ${
								activeStep === 1 || activeStep === 2 ? "bg-blue-500" : "bg-zinc-800"
							} transition-colors relative`}
						>
							<div
								className={`absolute bottom-0 -left-1 w-2 h-2 border-b border-r ${
									activeStep === 1 || activeStep === 2 ? "border-blue-500" : "border-zinc-800"
								} rotate-45`}
							/>
						</div>
						<div
							className={`px-4 py-2 rounded border ${
								activeStep === 2
									? "border-[#FF4500] text-[#FF4500] bg-[#FF4500]/10"
									: "border-zinc-700 text-zinc-500"
							} transition-colors text-xs font-mono`}
						>
							External DB
						</div>
					</div>
				</div>
				<p className="text-xs text-zinc-500 mt-4 text-center">
					State must be fetched from a remote DB for every request.
					<br />
					<span className="text-red-400">High Latency • Connection Limits</span>
				</p>
			</motion.div>

			{/* Rivet Actor */}
			<motion.div
				initial={{ opacity: 0, x: 20 }}
				whileInView={{ opacity: 1, x: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5 }}
				className="p-6 rounded-2xl border border-white/10 bg-zinc-900/30 backdrop-blur-md relative overflow-hidden shadow-[0_0_50px_-12px_rgba(16,185,129,0.1)] hover:shadow-[0_0_50px_-12px_rgba(16,185,129,0.2)] transition-shadow duration-500"
			>
				{/* Top Shine Highlight (Green tinted for the 'hero' card) */}
				<div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-emerald-500/40 to-transparent z-10" />
				<div className="absolute inset-0 bg-gradient-to-b from-emerald-500/5 to-transparent pointer-events-none" />
				<div className="flex items-center gap-2 mb-6 relative z-10">
					<div className="p-2 rounded bg-emerald-500/10 text-emerald-500">
						<Box className="w-4 h-4" />
					</div>
					<h4 className="font-semibold text-white">Rivet Actor Model</h4>
				</div>

				<div className="relative h-48 flex items-center justify-center">
					<div className="flex items-center gap-8 w-full justify-center">
						<div
							className={`px-4 py-2 rounded border ${
								activeStep % 2 === 0
									? "border-white bg-white text-black"
									: "border-zinc-700 text-zinc-500"
							} transition-colors text-xs font-mono`}
						>
							Client
						</div>
						<div
							className={`h-[1px] w-16 ${
								activeStep % 2 === 0 ? "bg-white" : "bg-zinc-800"
							} transition-colors relative`}
						>
							<div
								className={`absolute -top-1 right-0 w-2 h-2 border-t border-r ${
									activeStep % 2 === 0 ? "border-white" : "border-zinc-800"
								} rotate-45`}
							/>
							<div
								className={`absolute -top-1 left-0 w-2 h-2 border-l border-b ${
									activeStep % 2 === 0 ? "border-white" : "border-zinc-800"
								} rotate-45`}
							/>
						</div>

						{/* The Actor */}
						<div className="relative">
							<div
								className={`w-32 h-32 rounded-xl border ${
									activeStep % 2 !== 0
										? "border-emerald-500 bg-emerald-500/10"
										: "border-zinc-700 bg-zinc-900/50"
								} transition-all flex flex-col items-center justify-center gap-2`}
							>
								<div className="text-xs font-mono font-bold text-white">Actor</div>
								<div className="w-full h-[1px] bg-white/10" />
								<div className="flex items-center gap-2">
									<Cpu className="w-3 h-3 text-zinc-400" />
									<span className="text-[10px] text-zinc-400">Compute</span>
								</div>
								<div className="flex items-center gap-2">
									<Database className="w-3 h-3 text-emerald-500" />
									<span className="text-[10px] text-emerald-500">In-Mem State</span>
								</div>
							</div>
							{/* Pulse effect */}
							{activeStep % 2 !== 0 && (
								<div className="absolute inset-0 rounded-xl border border-emerald-500 animate-ping opacity-20" />
							)}
						</div>
					</div>
				</div>
				<p className="text-xs text-zinc-500 mt-4 text-center">
					State lives <i>with</i> the compute in memory.
					<br />
					<span className="text-emerald-500">Zero Latency • Realtime • Persistent</span>
				</p>
			</motion.div>
		</div>
	);
};

export const ConceptSection = () => (
	<section id="actors" className="py-32 bg-zinc-900/10 border-y border-white/5">
		<div className="max-w-7xl mx-auto px-6">
			<div className="flex flex-col md:flex-row gap-16">
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="flex-1"
				>
					<h2 className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight">
						Think in Actors,
						<br />
						not just Functions.
					</h2>
					<p className="text-lg text-zinc-400 leading-relaxed mb-6">
						<strong className="text-white">What is an Actor?</strong>
						<br />
						An Actor is a tiny, isolated server that holds its own data in memory. Unlike a stateless
						function that forgets everything after it runs, an Actor remembers.
					</p>
					<p className="text-lg text-zinc-400 leading-relaxed">
						<strong className="text-white">Why use them?</strong>
						<br />
						When you need to manage state for a specific entity—like a Chat Room, a Game Match, or a User
						Session—fetching that data from a database every 100ms is slow and expensive. Actors keep
						that data in RAM, right next to your logic.
					</p>
				</motion.div>
				<div className="flex-1">
					<ArchitectureComparison />
				</div>
			</div>
		</div>
	</section>
);

