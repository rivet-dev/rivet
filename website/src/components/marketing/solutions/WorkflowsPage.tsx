"use client";

import { useState, useEffect } from "react";
import {
	Terminal,
	Zap,
	Globe,
	ArrowRight,
	Box,
	Database,
	Check,
	Cpu,
	RefreshCw,
	Clock,
	Cloud,
	LayoutGrid,
	Activity,
	Wifi,
	AlertCircle,
	Gamepad2,
	MessageSquare,
	Bot,
	Users,
	FileText,
	Workflow,
	Gauge,
	Eye,
	Brain,
	Sparkles,
	Network,
	Calendar,
	GitBranch,
	Timer,
	Mail,
	CreditCard,
	Bell,
} from "lucide-react";
import { motion } from "framer-motion";

// --- Shared Design Components ---
const Badge = ({ text, color = "emerald" }) => {
	const colorClasses = {
		orange: "text-orange-400 border-orange-500/20 bg-orange-500/10",
		blue: "text-blue-400 border-blue-500/20 bg-blue-500/10",
		purple: "text-purple-400 border-purple-500/20 bg-purple-500/10",
		emerald: "text-emerald-400 border-emerald-500/20 bg-emerald-500/10",
	};

	return (
		<div
			className={`inline-flex items-center gap-2 px-3 py-1 rounded-full border text-xs font-medium mb-8 transition-colors cursor-default ${colorClasses[color]}`}
		>
			<span className={`w-1.5 h-1.5 rounded-full ${color === "orange" ? "bg-orange-400" : color === "blue" ? "bg-blue-400" : color === "purple" ? "bg-purple-400" : "bg-emerald-400"} animate-pulse`} />
			{text}
		</div>
	);
};

const CodeBlock = ({ code, fileName = "workflow.ts" }) => {
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
										// Simple custom tokenizer for this snippet
										const tokens = [];
										let current = line;

										// Handle comments first (consume rest of line)
										const commentIndex = current.indexOf("//");
										let comment = "";
										if (commentIndex !== -1) {
											comment = current.slice(commentIndex);
											current = current.slice(0, commentIndex);
										}

										// Split remaining code by delimiters but keep them
										const parts = current.split(/([a-zA-Z0-9_$]+|"[^"]*"|'[^']*'|\s+|[(){},.;:[\]])/g).filter(Boolean);

										parts.forEach((part, j) => {
											const trimmed = part.trim();

											// Keywords
											if (["import", "from", "export", "const", "return", "async", "await", "function", "let", "var", "if", "else"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-purple-400">{part}</span>);
											}
											// Functions & Special Rivet Terms
											else if (["actor", "broadcast", "schedule", "after", "spawn", "rpc", "sendEmail"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-400">{part}</span>);
											}
											// Object Keys / Properties / Methods
											else if (["state", "actions", "step", "userId", "start", "hasLoggedIn", "checkStatus", "markLoggedIn", "complete"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-300">{part}</span>);
											}
											// Strings
											else if (part.startsWith('"') || part.startsWith("'")) {
												tokens.push(<span key={j} className="text-[#FF4500]">{part}</span>);
											}
											// Numbers
											else if (!isNaN(Number(trimmed)) && trimmed !== "") {
												tokens.push(<span key={j} className="text-emerald-400">{part}</span>);
											}
											// Default (punctuation, variables like 'c', etc)
											else {
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

// --- Refined Workflow Card matching landing page style with color highlights ---
const SolutionCard = ({ title, description, icon: Icon, color = "emerald" }) => {
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
			case "purple":
				return {
					bg: "bg-purple-500/10",
					text: "text-purple-400",
					hoverBg: "group-hover:bg-purple-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(168,85,247,0.5)]",
					border: "border-purple-500",
					glow: "rgba(168,85,247,0.15)",
				};
			case "emerald":
				return {
					bg: "bg-emerald-500/10",
					text: "text-emerald-400",
					hoverBg: "group-hover:bg-emerald-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(16,185,129,0.5)]",
					border: "border-emerald-500",
					glow: "rgba(16,185,129,0.15)",
				};
			default:
				return {
					bg: "bg-emerald-500/10",
					text: "text-emerald-400",
					hoverBg: "group-hover:bg-emerald-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(16,185,129,0.5)]",
					border: "border-emerald-500",
					glow: "rgba(16,185,129,0.15)",
				};
		}
	};
	const c = getColorClasses(color);

	return (
		<div className="group relative overflow-hidden rounded-xl border border-white/10 bg-white/[0.02] hover:bg-white/[0.05] backdrop-blur-sm transition-all duration-300 flex flex-col h-full hover:border-white/20 hover:shadow-[0_0_30px_-10px_rgba(255,255,255,0.1)]">
			{/* Top Shine Highlight */}
			<div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent opacity-50 group-hover:opacity-100 transition-opacity z-10" />

			{/* Top Left Reflection/Glow */}
			<div
				className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none"
				style={{
					background: `radial-gradient(circle at top left, ${c.glow} 0%, transparent 50%)`,
				}}
			/>
			{/* Sharp Edge Highlight (Masked) */}
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

// --- Page Sections ---
const Hero = () => (
	<section className="relative pt-32 pb-20 md:pt-48 md:pb-32 overflow-hidden">
		<div className="absolute top-0 left-1/2 -translate-x-1/2 w-[1000px] h-[500px] bg-emerald-500/[0.03] blur-[100px] rounded-full pointer-events-none" />

		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<Badge text="Durable Execution" color="emerald" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="text-5xl md:text-7xl font-medium text-white tracking-tight leading-[1.1] mb-6"
					>
						Workflows that <br />
						<span className="text-emerald-400">Never Fail.</span>
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg md:text-xl text-zinc-400 leading-relaxed mb-8 max-w-lg"
					>
						Replace complex queues and state machines with simple code. Rivet Actors persist their execution state to disk, surviving server restarts and sleeping for months without resources.
					</motion.p>
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="flex flex-col sm:flex-row items-center gap-4"
					>
						<a href="https://dashboard.rivet.dev/" className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black shadow-sm hover:bg-zinc-200 transition-colors gap-2">
							Get Started
							<ArrowRight className="w-4 h-4" />
						</a>
						<a href="/docs/actors/schedule" className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white shadow-sm hover:border-white/20 transition-colors gap-2">
							Read the Docs
						</a>
					</motion.div>
				</div>
				<div className="flex-1 w-full max-w-xl">
					<div className="relative">
						<div className="absolute -inset-1 bg-gradient-to-r from-emerald-500/20 to-blue-500/20 rounded-xl blur opacity-40" />
						<CodeBlock
							fileName="onboarding.ts"
							code={`import { actor } from "rivetkit";

export const userOnboarding = actor({
  state: { step: 'welcome', hasLoggedIn: false },
  actions: {
    start: async (c) => {
      // 1. Send welcome email
      await sendEmail(c.state.userId, "welcome");

      // 2. Schedule nudge in 3 days (actor hibernates)
      c.schedule.after(3 * 24 * 60 * 60 * 1000, "checkStatus");
      c.state.step = "waiting";
    },

    checkStatus: async (c) => {
      // 3. Wake up and check if user has logged in
      if (!c.state.hasLoggedIn) {
        await sendEmail(c.state.userId, "nudge");
        // Schedule final check in 7 more days
        c.schedule.after(7 * 24 * 60 * 60 * 1000, "complete");
      }
    },

    markLoggedIn: (c) => { c.state.hasLoggedIn = true; }
  }
});`}
						/>
					</div>
				</div>
			</div>
		</div>
	</section>
);

const WorkflowArchitecture = () => {
	const [activeDay, setActiveDay] = useState(1);

	useEffect(() => {
		const interval = setInterval(() => {
			setActiveDay((d) => (d >= 4 ? 1 : d + 1));
		}, 2000);
		return () => clearInterval(interval);
	}, []);

	return (
		<section className="py-24 bg-black border-y border-white/5 relative">
			<div className="max-w-7xl mx-auto px-6">
				<div className="mb-16">
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						The Sleep/Wake Cycle
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-zinc-400 max-w-2xl text-lg leading-relaxed"
					>
						Unlike standard cron jobs, Actors maintain their exact execution pointer and local variable state across sleeps. They don't restart from the beginning; they continue.
					</motion.p>
				</div>

				<div className="grid lg:grid-cols-2 gap-12 items-center">
					{/* Visualization */}
					<div className="relative h-80 rounded-2xl border border-white/10 bg-zinc-900/20 flex flex-col items-center justify-center overflow-hidden p-8">

						{/* Timeline Container */}
						<div className="relative w-full z-10">
							<div className="flex items-start justify-between w-full">
								{/* Node 1: Start */}
								<div className="flex flex-col items-center gap-3 z-20 w-16">
									<div
										className={`w-12 h-12 rounded-full border-2 ${activeDay === 1 ? "border-emerald-500 bg-emerald-500/20 text-emerald-400" : "border-zinc-700 bg-zinc-900 text-zinc-500"} flex items-center justify-center transition-colors duration-500`}
									>
										<Zap className="w-5 h-5" />
									</div>
									<span className="text-xs font-mono text-zinc-400">Start</span>
								</div>

								{/* Spacer 1: Sleep */}
								<div className="flex-1 flex flex-col items-center relative px-4">
									{/* Track Line */}
									<div className="absolute top-6 left-0 right-0 h-[2px] bg-zinc-800 -translate-y-1/2" />

									{/* Progress Line */}
									<div className={`absolute top-6 left-0 right-0 h-[2px] bg-emerald-500/50 -translate-y-1/2 transition-all duration-1000 origin-left ${activeDay >= 2 ? "scale-x-100" : "scale-x-0"}`} />

									{/* Label floating above */}
									<div className={`mb-8 text-[10px] font-mono uppercase tracking-widest ${activeDay === 2 ? "text-emerald-500" : "text-zinc-600"} transition-colors`}>Hibernating</div>
								</div>

								{/* Node 2: Resume */}
								<div className="flex flex-col items-center gap-3 z-20 w-16">
									<div
										className={`w-12 h-12 rounded-full border-2 ${activeDay === 3 ? "border-emerald-500 bg-emerald-500/20 text-emerald-400" : "border-zinc-700 bg-zinc-900 text-zinc-500"} flex items-center justify-center transition-colors duration-500`}
									>
										<Bell className="w-5 h-5" />
									</div>
									<span className="text-xs font-mono text-zinc-400">Resume</span>
								</div>

								{/* Spacer 2: Short */}
								<div className="w-16 flex flex-col items-center relative mx-2">
									<div className="absolute top-6 left-0 right-0 h-[2px] bg-zinc-800 -translate-y-1/2" />
									<div className={`absolute top-6 left-0 right-0 h-[2px] bg-emerald-500/50 -translate-y-1/2 transition-all duration-500 origin-left ${activeDay >= 4 ? "scale-x-100" : "scale-x-0"}`} />
								</div>

								{/* Node 3: Done */}
								<div className="flex flex-col items-center gap-3 z-20 w-16">
									<div
										className={`w-12 h-12 rounded-full border-2 ${activeDay === 4 ? "border-emerald-500 bg-emerald-500/20 text-emerald-400" : "border-zinc-700 bg-zinc-900 text-zinc-500"} flex items-center justify-center transition-colors duration-500`}
									>
										<Check className="w-5 h-5" />
									</div>
									<span className="text-xs font-mono text-zinc-500">Done</span>
								</div>
							</div>
						</div>

						{/* Console Log Simulation */}
						<div className="mt-12 bg-black border border-zinc-800 rounded-lg p-3 font-mono text-xs text-zinc-400 w-full max-w-md shadow-xl z-20">
							<div className="flex items-center gap-2 mb-2 border-b border-zinc-800 pb-2">
								<div className="w-2 h-2 rounded-full bg-zinc-500/20 border border-zinc-500/50" />
								<div className="w-2 h-2 rounded-full bg-zinc-500/20 border border-zinc-500/50" />
								<div className="w-2 h-2 rounded-full bg-zinc-500/20 border border-zinc-500/50" />
								<span className="ml-auto text-zinc-600 text-[10px]">workflow_logs.txt</span>
							</div>
							<div className="space-y-1 h-20 overflow-hidden">
								<div className={`${activeDay >= 1 ? "opacity-100" : "opacity-20"} transition-opacity`}>
									<span className="text-zinc-500">[10:00:00]</span> <span className="text-emerald-400">INFO: Workflow started. sending_email...</span>
								</div>
								<div className={`${activeDay >= 2 ? "opacity-100" : "opacity-20"} transition-opacity`}>
									<span className="text-zinc-500">[10:00:01]</span> <span className="text-blue-400">SLEEP: Hibernating for 3 days...</span>
								</div>
								<div className={`${activeDay >= 3 ? "opacity-100" : "opacity-20"} transition-opacity`}>
									<span className="text-zinc-500">[+3d 00:00]</span> <span className="text-emerald-400">WAKE: Context restored from disk.</span>
								</div>
								<div className={`${activeDay >= 4 ? "opacity-100" : "opacity-20"} transition-opacity`}>
									<span className="text-zinc-500">[+3d 00:01]</span> <span className="text-zinc-300">SUCCESS: User logged in. Completing.</span>
								</div>
							</div>
						</div>
					</div>

					{/* Feature List */}
					<div className="space-y-6">
						<motion.div
							initial={{ opacity: 0, x: 20 }}
							whileInView={{ opacity: 1, x: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5 }}
							className="group"
						>
							<h3 className="text-xl font-medium text-white mb-2 flex items-center gap-2">
								<Database className="w-5 h-5 text-emerald-400" />
								Implicit State
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								Forget <code className="font-mono text-xs bg-zinc-900 px-1 py-0.5 rounded">UPDATE users SET status = 'emailed'</code>. Just define a variable in your code. Rivet persists the entire JS closure automatically.
							</p>
						</motion.div>
						<div className="w-full h-[1px] bg-white/5" />
						<motion.div
							initial={{ opacity: 0, x: 20 }}
							whileInView={{ opacity: 1, x: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="group"
						>
							<h3 className="text-xl font-medium text-white mb-2 flex items-center gap-2">
								<Clock className="w-5 h-5 text-blue-400" />
								Zero-Cost Waiting
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								When you <code className="font-mono text-xs bg-zinc-900 px-1 py-0.5 rounded">await sleep('1y')</code>, the Actor serializes to disk. You pay absolutely nothing for compute while it waits.
							</p>
						</motion.div>
						<div className="w-full h-[1px] bg-white/5" />
						<motion.div
							initial={{ opacity: 0, x: 20 }}
							whileInView={{ opacity: 1, x: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.2 }}
							className="group"
						>
							<h3 className="text-xl font-medium text-white mb-2 flex items-center gap-2">
								<RefreshCw className="w-5 h-5 text-purple-400" />
								Reliability Guarantees
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								If the server crashes or deploys during a sleep, the Actor wakes up on a healthy node as if nothing happened.
							</p>
						</motion.div>
					</div>
				</div>
			</div>
		</section>
	);
};

const WorkflowFeatures = () => {
	const features = [
		{
			title: "Durable Timers",
			description: "Schedule code to run in the future using natural language. '2 days', 'next friday', or specific ISO dates.",
			icon: Calendar,
			color: "emerald",
		},
		{
			title: "Human-in-the-Loop",
			description: "Pause execution until an external signal is received. Perfect for approval flows or 2FA verifications.",
			icon: Users,
			color: "blue",
		},
		{
			title: "Scheduled Jobs (Cron)",
			description: "Actors can be self-waking. Create a singleton actor that wakes up every hour to perform maintenance tasks.",
			icon: Clock,
			color: "orange",
		},
		{
			title: "Retries & Backoff",
			description: "Wrap flaky API calls in robust retry logic. If the process crashes, it resumes exactly where it failed.",
			icon: RefreshCw,
			color: "purple",
		},
		{
			title: "Sub-Workflows",
			description: "Spawn child actors to handle parallel tasks. The parent actor waits for results, aggregating data cleanly.",
			icon: GitBranch,
			color: "emerald",
		},
		{
			title: "State Inspection",
			description: "Debug running workflows by inspecting their memory state in real-time via the dashboard or REPL.",
			icon: Eye,
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
					className="mb-20 text-center"
				>
					<h2 className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight">Primitive for Reliability</h2>
					<p className="text-zinc-400 text-lg leading-relaxed">Building blocks for systems that must finish what they start.</p>
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

const UseCases = () => (
	<section className="py-24 bg-black border-t border-white/5">
		<div className="max-w-7xl mx-auto px-6">
			<div className="grid md:grid-cols-2 gap-16 items-center">
				<div>
					<Badge text="Case Study" color="emerald" />
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						Payment Dunning
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg text-zinc-400 mb-8 leading-relaxed"
					>
						Recover failed payments with a stateful actor that manages the entire lifecycle of the retry process.
					</motion.p>
					<ul className="space-y-4">
						{["Trigger: Stripe webhook spawns a DunningActor", "Logic: Wait 3 days, email user, retry charge", "End: If success, kill actor. If fail after 3 tries, cancel sub."].map((item, i) => (
							<motion.li
								key={i}
								initial={{ opacity: 0, x: -20 }}
								whileInView={{ opacity: 1, x: 0 }}
								viewport={{ once: true }}
								transition={{ duration: 0.5, delay: 0.2 + i * 0.1 }}
								className="flex items-center gap-3 text-zinc-300"
							>
								<div className="w-5 h-5 rounded-full bg-emerald-500/20 flex items-center justify-center">
									<Check className="w-3 h-3 text-emerald-400" />
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
					<div className="absolute inset-0 bg-gradient-to-r from-emerald-500/20 to-transparent rounded-2xl blur-2xl" />
					<div className="relative rounded-2xl border border-white/10 bg-zinc-900 p-6 shadow-2xl">
						<div className="flex items-center justify-between mb-6 border-b border-white/5 pb-4">
							<div className="flex items-center gap-3">
								<div className="w-8 h-8 rounded bg-emerald-500/20 flex items-center justify-center">
									<CreditCard className="w-5 h-5 text-emerald-400" />
								</div>
								<div>
									<div className="text-sm font-medium text-white">Invoice #INV-2049</div>
									<div className="text-xs text-zinc-500">Status: Retrying (Attempt 2/3)</div>
								</div>
							</div>
							<div className="px-2 py-1 rounded bg-yellow-500/10 text-yellow-400 text-xs border border-yellow-500/20">Pending</div>
						</div>
						<div className="space-y-4 text-sm font-mono">
							<div className="flex justify-between items-center text-zinc-500 text-xs">
								<span>Today</span>
								<span>Next Retry: 2d</span>
							</div>
							<div className="w-full bg-zinc-800 rounded-full h-1.5">
								<div className="bg-emerald-500 h-1.5 rounded-full" style={{ width: "66%" }} />
							</div>
							<div className="p-3 rounded bg-zinc-950 border border-white/5 text-zinc-400">Card declined. Email sent to user@example.com. Sleeping...</div>
						</div>
					</div>
				</motion.div>
			</div>
		</div>
	</section>
);

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
				Connects with
			</motion.h2>
			<div className="flex flex-wrap justify-center gap-4">
				{["Stripe", "Resend", "Twilio", "Slack", "Linear", "Postgres", "Discord"].map((tech, i) => (
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

export default function WorkflowsPage() {
	return (
		<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-emerald-500/30 selection:text-emerald-200">
			<main>
				<Hero />
				<WorkflowArchitecture />
				<WorkflowFeatures />
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
							Sleep well at night.
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-lg text-zinc-400 mb-10 leading-relaxed"
						>
							Trust your critical background processes to a runtime built for durability.
						</motion.p>
						<motion.div
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.2 }}
							className="flex flex-col sm:flex-row items-center justify-center gap-4"
						>
							<a href="https://dashboard.rivet.dev/" className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black shadow-sm hover:bg-zinc-200 transition-colors">
								Start Building
							</a>
							<a href="/docs/actors" className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white shadow-sm hover:border-white/20 transition-colors">
								Read the Docs
							</a>
						</motion.div>
					</div>
				</section>
			</main>
		</div>
	);
}

