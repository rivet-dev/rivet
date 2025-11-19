"use client";

import { useEffect, useState, useRef } from "react";
import { motion } from "framer-motion";

export const CodeWalkthrough = () => {
	const [activeStep, setActiveStep] = useState(0);
	const observerRefs = useRef([]);

	const steps = [
		{
			title: "Define the Actor",
			description:
				"Start by importing and exporting an actor definition. This creates a specialized serverless function that maintains its own isolated memory space.",
			lines: [0, 2, 14],
		},
		{
			title: "Declare Persistent State",
			description:
				"Define the shape of your data. This state object is automatically persisted to disk and loaded into memory when the actor wakes up. No database queries needed.",
			lines: [3],
		},
		{
			title: "Write RPC Actions",
			description:
				"Actions are just functions. They run directly in the actor's memory space with zero network latency to access the state.",
			lines: [5, 6, 12, 13],
		},
		{
			title: "Mutate State Directly",
			description:
				"Just modify the state variable. Rivet detects the changes and handles the persistence and replication for you.",
			lines: [7, 8],
		},
		{
			title: "Broadcast Realtime Events",
			description:
				"Push updates to all connected clients instantly using WebSockets. It's built right into the context object.",
			lines: [10],
		},
	];

	const codeLines = [
		`import { actor } from "rivetkit";`,
		``,
		`export const chatRoom = actor({`,
		`  state: { messages: [] },`,
		``,
		`  actions: {`,
		`    postMessage: (c, text) => {`,
		`      const msg = { text, at: Date.now() };`,
		`      c.state.messages.push(msg);`,
		`      `,
		`      c.broadcast("newMessage", msg);`,
		`      return "sent";`,
		`    }`,
		`  }`,
		`});`,
	];

	useEffect(() => {
		const options = {
			root: null,
			rootMargin: "-40% 0px -40% 0px",
			threshold: 0,
		};

		const observer = new IntersectionObserver((entries) => {
			entries.forEach((entry) => {
				if (entry.isIntersecting) {
					const index = Number(entry.target.dataset.index);
					setActiveStep(index);
				}
			});
		}, options);

		observerRefs.current.forEach((ref) => {
			if (ref) observer.observe(ref);
		});

		return () => observer.disconnect();
	}, []);

	return (
		<section className="py-24 bg-black relative border-t border-white/10">
			<div className="max-w-7xl mx-auto px-6">
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="mb-16"
				>
					<h2 className="text-3xl font-medium text-white mb-4 tracking-tight">How it works</h2>
					<p className="text-zinc-400 max-w-xl">
						Rivet makes backend development feel like frontend development. Define your state, write your
						logic, and let the engine handle the rest.
					</p>
				</motion.div>

				<div className="grid grid-cols-1 lg:grid-cols-2 gap-16">
					{/* Sticky Code Block */}
					<div className="hidden lg:block relative">
						<div className="sticky top-32">
							<div className="rounded-xl overflow-hidden border border-white/10 bg-zinc-900/50 backdrop-blur-xl shadow-2xl transition-all duration-500">
								<div className="flex items-center gap-2 px-4 py-3 border-b border-white/5 bg-white/5">
									<div className="flex gap-1.5">
										<div className="w-3 h-3 rounded-full bg-red-500/20 border border-red-500/50" />
										<div className="w-3 h-3 rounded-full bg-yellow-500/20 border border-yellow-500/50" />
										<div className="w-3 h-3 rounded-full bg-green-500/20 border border-green-500/50" />
									</div>
									<span className="text-xs text-zinc-500 font-mono ml-2">chat_room.ts</span>
								</div>
								<div className="p-6 font-mono text-sm leading-7 overflow-x-auto">
									{codeLines.map((line, idx) => {
										const isHighlighted = steps[activeStep].lines.includes(idx);
										const isDimmed = !isHighlighted;

										return (
											<div
												key={idx}
												className={`transition-all duration-500 flex ${
													isDimmed ? "opacity-30 blur-[1px]" : "opacity-100 scale-[1.01]"
												}`}
											>
												<span className="w-8 inline-block text-zinc-700 select-none text-right pr-4">
													{idx + 1}
												</span>
												<span
													className={`${isHighlighted ? "text-white font-medium" : "text-zinc-400"}`}
												>
													{line.split(/(\s+)/).map((part, i) => {
														if (
															part.trim() === "import" ||
															part.trim() === "export" ||
															part.trim() === "const" ||
															part.trim() === "return"
														)
															return (
																<span key={i} className="text-purple-400">
																	{part}
																</span>
															);
														if (part.trim() === "actor" || part.trim() === "broadcast")
															return (
																<span key={i} className="text-blue-400">
																	{part}
																</span>
															);
														if (part.includes('"'))
															return (
																<span key={i} className="text-[#FF4500]">
																	{part}
																</span>
															);
														if (part.includes("//"))
															return (
																<span key={i} className="text-zinc-500">
																	{part}
																</span>
															);
														return part;
													})}
												</span>
											</div>
										);
									})}
								</div>
							</div>
						</div>
					</div>

					{/* Scrolling Steps */}
					<div className="space-y-32 py-12">
						{steps.map((step, idx) => (
							<div
								key={idx}
								data-index={idx}
								ref={(el) => (observerRefs.current[idx] = el)}
								className={`transition-all duration-500 p-6 rounded-2xl border backdrop-blur-sm ${
									idx === activeStep
										? "bg-white/[0.04] border-white/20 shadow-[0_0_30px_-10px_rgba(255,255,255,0.1)]"
										: "bg-white/[0.02] border-white/5 opacity-50 hover:opacity-100 hover:bg-white/[0.05] hover:border-white/10"
								}`}
							>
								<div className="flex items-center gap-3 mb-4">
									<div
										className={`w-8 h-8 rounded-lg flex items-center justify-center text-sm font-bold transition-colors ${
											idx === activeStep ? "bg-white/10 text-white border border-white/20" : "bg-zinc-800 text-zinc-500"
										}`}
									>
										{idx + 1}
									</div>
									<h3
										className={`text-xl font-medium transition-colors ${
											idx === activeStep ? "text-white" : "text-zinc-500"
										}`}
									>
										{step.title}
									</h3>
								</div>
								<p className="text-zinc-400 leading-relaxed text-lg">{step.description}</p>

								{/* Mobile Only Code Snippet */}
								<div className="lg:hidden mt-6 bg-[#0A0A0A] rounded-lg p-4 border border-white/10 font-mono text-xs text-zinc-300 overflow-x-auto">
									{step.lines.map((lineIdx) => (
										<div key={lineIdx} className="whitespace-pre">
											{codeLines[lineIdx]}
										</div>
									))}
								</div>
							</div>
						))}
						<div className="h-[20vh]" />
					</div>
				</div>
			</div>
		</section>
	);
};

