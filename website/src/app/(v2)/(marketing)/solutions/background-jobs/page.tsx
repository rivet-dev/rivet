"use client";

import { useState } from "react";
import {
	Terminal,
	Zap,
	Globe,
	ArrowRight,
	Box,
	Database,
	Layers,
	Check,
	Cpu,
	RefreshCw,
	Clock,
	Shield,
	Cloud,
	Activity,
	Workflow,
	AlertTriangle,
	Archive,
	Gauge,
	Play,
	Link as LinkIcon,
} from "lucide-react";
import { motion } from "framer-motion";

// --- Shared Design Components ---
const Badge = ({ text, color = "orange" }) => {
	const colorClasses = {
		orange: "text-orange-400 border-orange-500/20 bg-orange-500/10",
		blue: "text-blue-400 border-blue-500/20 bg-blue-500/10",
		red: "text-red-400 border-red-500/20 bg-red-500/10",
		zinc: "text-zinc-400 border-zinc-500/20 bg-zinc-500/10",
	};

	return (
		<div
			className={`inline-flex items-center gap-2 px-3 py-1 rounded-full border text-xs font-medium mb-8 transition-colors cursor-default ${colorClasses[color]}`}
		>
			<span className={`w-1.5 h-1.5 rounded-full ${color === "orange" ? "bg-orange-400" : color === "blue" ? "bg-blue-400" : color === "red" ? "bg-red-400" : "bg-zinc-400"} animate-pulse`} />
			{text}
		</div>
	);
};

const CodeBlock = ({ code, fileName = "worker.ts" }) => {
	return (
		<div className="relative group rounded-xl overflow-hidden border border-white/10 bg-black shadow-2xl">
			<div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-white/30 to-transparent z-10" />
			<div className="flex items-center justify-between px-4 py-3 border-b border-white/5 bg-white/5">
				<div className="flex items-center gap-2">
					<div className="w-3 h-3 rounded-full bg-zinc-500/20 border border-zinc-500/50" />
					<div className="w-3 h-3 rounded-full bg-zinc-500/20 border border-zinc-500/50" />
					<div className="w-3 h-3 rounded-full bg-zinc-500/20 border border-zinc-500/50" />
				</div>
				<div className="text-xs text-zinc-500 font-mono">{fileName}</div>
			</div>
			<div className="p-4 overflow-x-auto scrollbar-hide">
				<pre className="text-sm font-mono leading-relaxed text-zinc-300">
					<code>
						{code.split("\n").map((line, i) => (
							<div key={i} className="table-row">
								<span className="table-cell select-none text-right pr-4 text-zinc-700 w-8">
									{i + 1}
								</span>
								<span className="table-cell">
									{(() => {
										const tokens = [];
										let current = line;

										const commentIndex = current.indexOf("//");
										let comment = "";
										if (commentIndex !== -1) {
											comment = current.slice(commentIndex);
											current = current.slice(0, commentIndex);
										}

										const parts = current.split(/([a-zA-Z0-9_$]+|"[^"]*"|'[^']*'|\s+|[(){},.;:[\]])/g).filter(Boolean);

										parts.forEach((part, j) => {
											const trimmed = part.trim();

											if (["import", "from", "export", "const", "return", "async", "await", "try", "catch", "if"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-purple-400">{part}</span>);
											} else if (["actor", "schedule", "sendEmail", "enqueue", "process"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-400">{part}</span>);
											} else if (["state", "actions", "queue", "job", "attempts", "err", "delay", "shift", "push"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-300">{part}</span>);
											} else if (part.startsWith('"') || part.startsWith("'")) {
												tokens.push(<span key={j} className="text-[#FF4500]">{part}</span>);
											} else if (!isNaN(Number(trimmed)) && trimmed !== "") {
												tokens.push(<span key={j} className="text-emerald-400">{part}</span>);
											} else {
												tokens.push(<span key={j} className="text-zinc-300">{part}</span>);
											}
										});

										if (comment) {
											tokens.push(<span key="comment" className="text-zinc-500">{comment}</span>);
										}

										return tokens;
									})()}
								</span>
							</div>
						))}
					</code>
				</pre>
			</div>
		</div>
	);
};

const SolutionCard = ({ title, description, icon: Icon, color = "orange" }) => {
	const getColorClasses = (col) => {
		switch (col) {
			case "orange":
				return {
					bg: "bg-orange-500/10",
					text: "text-orange-400",
					hoverBg: "group-hover:bg-orange-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(249,115,22,0.5)]",
					border: "border-orange-500",
					glow: "rgba(249,115,22,0.15)",
				};
			case "blue":
				return {
					bg: "bg-blue-500/10",
					text: "text-blue-400",
					hoverBg: "group-hover:bg-blue-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(59,130,246,0.5)]",
					border: "border-blue-500",
					glow: "rgba(59,130,246,0.15)",
				};
			case "red":
				return {
					bg: "bg-red-500/10",
					text: "text-red-400",
					hoverBg: "group-hover:bg-red-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(239,68,68,0.5)]",
					border: "border-red-500",
					glow: "rgba(239,68,68,0.15)",
				};
			case "zinc":
				return {
					bg: "bg-zinc-500/10",
					text: "text-zinc-400",
					hoverBg: "group-hover:bg-zinc-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(113,113,122,0.5)]",
					border: "border-zinc-500",
					glow: "rgba(113,113,122,0.15)",
				};
			default:
				return {
					bg: "bg-orange-500/10",
					text: "text-orange-400",
					hoverBg: "group-hover:bg-orange-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(249,115,22,0.5)]",
					border: "border-orange-500",
					glow: "rgba(249,115,22,0.15)",
				};
		}
	};
	const c = getColorClasses(color);

	return (
		<div className="group relative overflow-hidden rounded-xl border border-white/10 bg-white/[0.02] hover:bg-white/[0.05] backdrop-blur-sm transition-all duration-300 flex flex-col h-full hover:border-white/20 hover:shadow-[0_0_30px_-10px_rgba(255,255,255,0.1)]">
			<div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent opacity-50 group-hover:opacity-100 transition-opacity z-10" />
			<div
				className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none"
				style={{
					background: `radial-gradient(circle at top left, ${c.glow} 0%, transparent 50%)`,
				}}
			/>
			<div className={`absolute top-0 left-0 w-24 h-24 rounded-tl-xl border-t border-l ${c.border} opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none z-20 [mask-image:linear-gradient(135deg,black_0%,transparent_50%)]`} />

			<div className="p-6 flex flex-col flex-grow relative z-10">
				<div className="flex items-center justify-between mb-4">
					<div className="flex items-center gap-3">
						<div className={`p-2 rounded ${c.bg} ${c.text} ${c.hoverBg} ${c.hoverShadow} transition-all duration-500`}>
							<Icon className="w-5 h-5" />
						</div>
						<h3 className="font-medium text-white tracking-tight">{title}</h3>
					</div>
				</div>
				<p className="text-sm text-zinc-400 leading-relaxed flex-grow">{description}</p>
			</div>
		</div>
	);
};

const Hero = () => (
	<section className="relative pt-32 pb-20 md:pt-48 md:pb-32 overflow-hidden">
		<div className="absolute top-0 left-1/2 -translate-x-1/2 w-[1000px] h-[500px] bg-orange-500/[0.03] blur-[100px] rounded-full pointer-events-none" />

		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<Badge text="Serverless Workers" color="orange" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="text-5xl md:text-7xl font-medium text-white tracking-tight leading-[1.1] mb-6"
					>
						Background Jobs <br />
						<span className="text-orange-400">Redefined.</span>
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg md:text-xl text-zinc-400 leading-relaxed mb-8 max-w-lg"
					>
						Forget Redis, SQS, and worker fleets. Rivet Actors combine the queue and the worker into a single, persistent entity. Schedule tasks, retry failures, and sleep for free.
					</motion.p>

					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="flex flex-col sm:flex-row items-center gap-4"
					>
						<a href="/docs" className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black shadow-sm hover:bg-zinc-200 transition-colors gap-2">
							Get Started
							<ArrowRight className="w-4 h-4" />
						</a>
					</motion.div>
				</div>

				<div className="flex-1 w-full max-w-xl">
					<div className="relative">
						<div className="absolute -inset-1 bg-gradient-to-r from-orange-500/20 to-red-500/20 rounded-xl blur opacity-40" />
						<CodeBlock
							fileName="email_worker.ts"
							code={`import { actor } from "rivetkit";

export const emailWorker = actor({
  // Persistent job queue in memory
  state: { queue: [] },

  actions: {
    enqueue: (c, job) => {
      c.state.queue.push({ ...job, attempts: 0 });
      c.schedule("process", "immediate");
    },

    process: async (c) => {
      const job = c.state.queue.shift();
      if (!job) return;

      try {
        await sendEmail(job);
      } catch (err) {
        job.attempts++;
        // Retry with exponential backoff
        const delay = Math.pow(2, job.attempts) + "s";
        c.state.queue.push(job);
        c.schedule("process", delay);
      }
    }
  }
});`}
						/>
					</div>
				</div>
			</div>
		</div>
	</section>
);

const QueueArchitecture = () => {
	return (
		<section className="py-24 bg-black border-y border-white/5 relative">
			<div className="max-w-7xl mx-auto px-6">
				<div className="mb-16 text-center">
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						The Actor as a Worker
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-zinc-400 max-w-2xl mx-auto text-lg leading-relaxed"
					>
						No need for a separate worker fleet. Every actor can schedule its own tasks, sleep while waiting, and wake up to retry failures.
					</motion.p>
				</div>

				<div className="relative h-[450px] w-full rounded-2xl border border-white/10 bg-zinc-900/20 flex items-center justify-center overflow-hidden p-8">
					<div className="absolute inset-0 bg-[linear-gradient(rgba(255,255,255,0.03)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.03)_1px,transparent_1px)] bg-[size:40px_40px]" />

					<div className="relative z-10 w-full max-w-6xl flex items-center justify-between gap-12 px-8">
						{/* Input Stream */}
						<div className="flex flex-col items-center gap-4 relative z-20 flex-shrink-0">
							<div className="w-20 h-20 rounded-xl bg-zinc-950 border border-zinc-700 flex items-center justify-center shadow-lg relative">
								<div className="absolute -top-3 px-2 py-0.5 bg-zinc-800 text-[10px] rounded-full text-zinc-400 border border-zinc-700">Source</div>
								<Cloud className="w-8 h-8 text-zinc-400" />
							</div>
							{/* Emitting Jobs Animation */}
							<div className="absolute top-1/2 left-full w-8 h-8 flex items-center justify-center">
								<div className="w-3 h-3 bg-orange-500/50 rounded animate-ping absolute" />
								<div className="w-3 h-3 bg-orange-500/50 rounded animate-ping delay-300 absolute" style={{ animationDelay: '300ms' }} />
							</div>
						</div>

						{/* The Queue Pipe */}
						<div className="flex-1 h-32 bg-zinc-950/80 border border-zinc-800 rounded-2xl relative overflow-hidden flex items-center px-4 gap-4 shadow-[inset_0_0_20px_rgba(0,0,0,0.5)]">
							<div className="absolute top-3 left-4 text-[10px] text-zinc-600 font-mono flex items-center gap-2">
								<Workflow className="w-3 h-3" />
								IN-MEMORY QUEUE
							</div>

							{/* Background Flow Animation */}
							<div className="absolute inset-0 opacity-20">
								<div className="absolute inset-0 bg-[linear-gradient(90deg,transparent,rgba(249,115,22,0.1),transparent)] animate-[flow_3s_linear_infinite]" />
							</div>

							{/* Jobs Animation Container - Using pure CSS traverse for smoothness */}
							<div className="flex-1 h-full relative overflow-hidden">
								{[0, 1, 2, 3, 4].map((i) => (
									<div
										key={i}
										className="absolute top-1/2 -translate-y-1/2 h-16 w-16 rounded-xl border bg-zinc-900 border-zinc-700 flex flex-col items-center justify-center gap-2 shadow-md animate-[traverse_5s_linear_infinite]"
										style={{
											animationDelay: `${i * 1}s`,
										}}
									>
										<Box className="w-6 h-6 text-orange-400" />
										<div className="w-10 h-1.5 bg-zinc-800 rounded-full overflow-hidden">
											<div
												className="h-full bg-orange-500/50 w-full animate-[progress_5s_linear_infinite]"
												style={{ animationDelay: `${i * 1}s` }}
											/>
										</div>
									</div>
								))}
							</div>
						</div>

						{/* The Worker (Actor) */}
						<div className="flex flex-col items-center gap-4 relative z-20 flex-shrink-0">
							<div className="w-32 h-32 rounded-full bg-black border-2 border-orange-500/50 flex flex-col items-center justify-center shadow-[0_0_50px_rgba(249,115,22,0.3)] relative z-10 overflow-hidden group">
								{/* Pulsing Core */}
								<div className="absolute inset-0 bg-orange-500/10 animate-pulse" />
								<div className="absolute inset-2 rounded-full border border-orange-500/20 animate-[spin_10s_linear_infinite]" />

								<Cpu className="w-12 h-12 text-orange-500 mb-1 relative z-10 animate-[pulse_2s_ease-in-out_infinite]" />
								<span className="text-xs text-orange-300 font-mono relative z-10">Processing</span>

								{/* Job Ingestion Effect */}
								<div className="absolute left-0 top-1/2 -translate-y-1/2 w-8 h-8 bg-orange-500/20 rounded-full blur-md animate-[ingest_4s_linear_infinite] opacity-0" />
							</div>
							{/* Connection Beam */}
							<div className="absolute top-1/2 right-full w-8 h-[2px] bg-gradient-to-r from-transparent to-orange-500/50" />
						</div>
					</div>
				</div>
				<style>{`
					@keyframes flow {
						0% { transform: translateX(-100%); }
						100% { transform: translateX(100%); }
					}
					@keyframes traverse {
						0% { left: -80px; opacity: 0; transform: translateY(-50%) scale(0.8); }
						10% { opacity: 1; transform: translateY(-50%) scale(1); }
						90% { opacity: 1; transform: translateY(-50%) scale(1); }
						100% { left: 100%; opacity: 0; transform: translateY(-50%) scale(0.8); }
					}
					@keyframes progress {
						0% { width: 0%; }
						100% { width: 100%; }
					}
					@keyframes ingest {
						0%, 80% { opacity: 0; transform: translate(-20px, -50%) scale(0.5); }
						90% { opacity: 1; transform: translate(0, -50%) scale(1.2); }
						100% { opacity: 0; transform: translate(10px, -50%) scale(0.5); }
					}
				`}</style>
			</div>
		</section>
	);
};

const JobFeatures = () => {
	const features = [
		{
			title: "Cron Schedules",
			description: "Built-in cron support. Wake up an actor every hour, day, or month to perform maintenance or reporting.",
			icon: Clock,
			color: "orange",
		},
		{
			title: "Exponential Backoff",
			description: "Network flaked? API down? Easily implement robust retry logic with increasing delays between attempts.",
			icon: RefreshCw,
			color: "blue",
		},
		{
			title: "Dead Letter Queues",
			description: "Move failed jobs to a separate list in state for manual inspection after max retries are exhausted.",
			icon: Archive,
			color: "red",
		},
		{
			title: "Rate Limiting",
			description: "Protect downstream APIs. Use a token bucket in actor memory to strictly limit outgoing request rates.",
			icon: Gauge,
			color: "zinc",
		},
		{
			title: "Batching",
			description: "Accumulate webhooks or events in memory and flush them to your database in a single bulk insert.",
			icon: Layers,
			color: "orange",
		},
		{
			title: "Job Prioritization",
			description: "Use multiple lists (high, normal, low) within your actor state to ensure critical tasks run first.",
			icon: AlertTriangle,
			color: "blue",
		},
	];

	return (
		<section className="py-32 bg-zinc-900/20 relative">
			<div className="max-w-7xl mx-auto px-6">
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="mb-20"
				>
					<h2 className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight">Complete Job Control</h2>
					<p className="text-zinc-400 text-lg leading-relaxed">Primitives for building reliable background systems.</p>
				</motion.div>

				<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
					{features.map((feat, idx) => (
						<motion.div
							key={idx}
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: idx * 0.05 }}
						>
							<SolutionCard {...feat} />
						</motion.div>
					))}
				</div>
			</div>
		</section>
	);
};

const CaseStudy = () => (
	<section className="py-24 bg-black border-t border-white/5">
		<div className="max-w-7xl mx-auto px-6">
			<div className="grid md:grid-cols-2 gap-16 items-center">
				<div>
					<Badge text="Case Study" color="orange" />
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						Video Transcoding Pipeline
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg text-zinc-400 mb-8 leading-relaxed"
					>
						Coordinate a complex, multi-step media processing workflow without managing intermediate state in a database.
					</motion.p>
					<ul className="space-y-4">
						{["Step 1: Upload triggers Actor creation", "Step 2: Actor calls external Transcoder API", "Step 3: Actor sleeps until webhook callback received", "Step 4: Notify user via WebSocket"].map((item, i) => (
							<motion.li
								key={i}
								initial={{ opacity: 0, x: -20 }}
								whileInView={{ opacity: 1, x: 0 }}
								viewport={{ once: true }}
								transition={{ duration: 0.5, delay: 0.2 + i * 0.1 }}
								className="flex items-center gap-3 text-zinc-300"
							>
								<div className="w-5 h-5 rounded-full bg-orange-500/20 flex items-center justify-center">
									<Check className="w-3 h-3 text-orange-400" />
								</div>
								{item}
							</motion.li>
						))}
					</ul>
				</div>
				<motion.div
					initial={{ opacity: 0, x: 20 }}
					whileInView={{ opacity: 1, x: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="relative"
				>
					<div className="absolute inset-0 bg-gradient-to-r from-orange-500/20 to-transparent rounded-2xl blur-2xl" />
					<div className="relative rounded-2xl border border-white/10 bg-zinc-900 p-6 shadow-2xl">
						<div className="flex items-center justify-between mb-6 border-b border-white/5 pb-4">
							<div className="flex items-center gap-3">
								<div className="w-8 h-8 rounded bg-orange-500/20 flex items-center justify-center">
									<Play className="w-5 h-5 text-orange-400" />
								</div>
								<div>
									<div className="text-sm font-medium text-white">Job: 1080p_Render</div>
									<div className="text-xs text-zinc-500">Duration: 4m 12s</div>
								</div>
							</div>
							<div className="px-2 py-1 rounded bg-yellow-500/10 text-yellow-400 text-xs border border-yellow-500/20">Waiting</div>
						</div>
						<div className="space-y-4 text-sm font-mono">
							<div className="p-3 rounded bg-zinc-950 border border-white/5 text-zinc-400">&gt; Transcode started. Sleeping...</div>
							<div className="p-3 rounded bg-zinc-950 border border-white/5 text-zinc-500 opacity-50">(Actor Hibernated to Disk)</div>
							<div className="p-3 rounded bg-orange-900/20 border border-orange-500/30 text-orange-200 animate-pulse">&lt; Webhook received. Waking up!</div>
						</div>
					</div>
				</motion.div>
			</div>
		</div>
	</section>
);

const UseCases = () => {
	const cases = [
		{
			title: "Email Drip Campaigns",
			desc: "Schedule a sequence of emails for a user. Sleep for days between sends. Store user progress in state.",
		},
		{
			title: "Report Generation",
			desc: "Trigger a heavy aggregation job. Poll the database, build the PDF, and email it when done.",
		},
		{
			title: "AI Batch Processing",
			desc: "Queue thousands of prompts for LLM processing. Rate limit requests to avoid API bans.",
		},
		{
			title: "Webhook Ingestion",
			desc: "Buffer high-velocity webhooks (e.g. Stripe events) in memory and process them reliably in order.",
		},
	];

	return (
		<section className="py-24 bg-black border-t border-white/5">
			<div className="max-w-7xl mx-auto px-6">
				<motion.h2
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="text-3xl md:text-5xl font-medium text-white mb-12 text-center tracking-tight"
				>
					Built for Reliability
				</motion.h2>
				<div className="grid md:grid-cols-2 lg:grid-cols-4 gap-6">
					{cases.map((c, i) => (
						<motion.div
							key={i}
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: i * 0.05 }}
							className="p-6 rounded-xl border border-white/10 bg-zinc-900/30 hover:bg-orange-900/10 hover:border-orange-500/30 transition-colors group"
						>
							<div className="mb-4">
								<Activity className="w-6 h-6 text-orange-500 group-hover:scale-110 transition-transform" />
							</div>
							<h4 className="text-white font-medium mb-2">{c.title}</h4>
							<p className="text-sm text-zinc-400">{c.desc}</p>
						</motion.div>
					))}
				</div>
			</div>
		</section>
	);
};

const Ecosystem = () => (
	<section className="py-24 bg-zinc-900/20 border-t border-white/5 relative overflow-hidden">
		<div className="max-w-7xl mx-auto px-6 relative z-10 text-center">
			<motion.h2
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5 }}
				className="text-3xl md:text-5xl font-medium text-white mb-12 tracking-tight"
			>
				Works with your stack
			</motion.h2>
			<div className="flex flex-wrap justify-center gap-4">
				{["Stripe", "Resend", "Twilio", "Slack", "OpenAI", "Postgres"].map((tech, i) => (
					<motion.div
						key={tech}
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: i * 0.05 }}
						className="px-6 py-3 rounded-xl border border-white/10 bg-black/50 text-zinc-400 text-sm font-mono hover:text-white hover:border-white/30 transition-colors cursor-default backdrop-blur-sm"
					>
						{tech}
					</motion.div>
				))}
			</div>
		</div>
	</section>
);

export default function BackgroundJobsPage() {
	return (
		<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-orange-500/30 selection:text-orange-200">
			<main>
				<Hero />
				<QueueArchitecture />
				<JobFeatures />
				<CaseStudy />
				<UseCases />
				<Ecosystem />

				{/* CTA Section */}
				<section className="py-32 text-center px-6 border-t border-white/10 bg-gradient-to-b from-black to-zinc-900/50">
					<div className="max-w-3xl mx-auto">
						<motion.h2
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5 }}
							className="text-4xl md:text-5xl font-medium text-white mb-6 tracking-tight"
						>
							Fire and forget.
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-lg text-zinc-400 mb-10 leading-relaxed"
						>
							Start building reliable, self-healing background jobs with Rivet.
						</motion.p>
						<motion.div
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.2 }}
							className="flex flex-col sm:flex-row items-center justify-center gap-4"
						>
							<a href="/docs" className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black shadow-sm hover:bg-zinc-200 transition-colors">
								Start Building Now
							</a>
							<a href="/templates" className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white shadow-sm hover:border-white/20 transition-colors">
								View Examples
							</a>
						</motion.div>
					</div>
				</section>
			</main>
		</div>
	);
}

