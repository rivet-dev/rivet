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

const CodeBlock = ({ code, fileName = "workflow.ts" }) => {
	return (
		<div className="relative rounded-lg overflow-hidden border border-white/10 bg-black">
			<div className="flex items-center justify-between px-4 py-2 border-b border-white/5 bg-white/5">
				<div className="flex items-center gap-1.5">
					<div className="w-2.5 h-2.5 rounded-full bg-zinc-700" />
					<div className="w-2.5 h-2.5 rounded-full bg-zinc-700" />
					<div className="w-2.5 h-2.5 rounded-full bg-zinc-700" />
				</div>
				<div className="text-xs text-zinc-500 font-mono">{fileName}</div>
			</div>
			<div className="p-4 overflow-x-auto">
				<pre className="text-sm font-mono leading-relaxed text-zinc-300">
					<code>
						{code.split("\n").map((line, i) => (
							<div key={i} className="table-row">
								<span className="table-cell select-none text-right pr-4 text-zinc-600 w-8">
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

											if (["import", "from", "export", "const", "return", "async", "await", "function", "let", "var", "if", "else", "while", "true", "false"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-purple-400">{part}</span>);
											}
											else if (["actor", "schedule", "after", "spawn", "rpc", "sendEmail", "ai"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-400">{part}</span>);
											}
											else if (["state", "actions", "step", "userId", "start", "hasLoggedIn", "checkStatus", "markLoggedIn", "complete", "broadcast", "c", "run", "queue", "next", "push"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-300">{part}</span>);
											}
											else if (part.startsWith('"') || part.startsWith("'")) {
												tokens.push(<span key={j} className="text-[#FF4500]">{part}</span>);
											}
											else if (!isNaN(Number(trimmed)) && trimmed !== "") {
												tokens.push(<span key={j} className="text-purple-400">{part}</span>);
											}
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

// --- Page Sections ---
const Hero = () => (
	<section className="relative overflow-hidden pb-20 pt-32 md:pb-32 md:pt-48">
		<div className="mx-auto max-w-7xl px-6">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="mb-6"
					>
						<span className="inline-flex items-center gap-2 rounded-full border border-white/10 px-3 py-1 text-xs text-zinc-400">
							<span className="h-1.5 w-1.5 rounded-full bg-[#FF4500]" />
							Durable Execution
						</span>
					</motion.div>

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="mb-6 text-4xl font-normal leading-[1.1] tracking-tight text-white md:text-6xl"
					>
						Workflows that Never Fail
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="mb-8 max-w-lg text-base leading-relaxed text-zinc-500"
					>
						Replace complex queues and state machines with simple code. Rivet Actors persist their execution state to disk, surviving server restarts and sleeping for months without resources.
					</motion.p>
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.3 }}
						className="flex flex-col sm:flex-row items-center gap-3"
					>
						<a href="/docs" className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200">
							Start Building
							<ArrowRight className="h-4 w-4" />
						</a>
						<a href="/docs/actors/schedule" className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white">
							Read the Docs
						</a>
					</motion.div>
				</div>
				<div className="flex-1 w-full max-w-xl">
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
		<section className="border-y border-white/10 py-48">
			<div className="mx-auto max-w-7xl px-6">
				<div className="flex flex-col gap-12">
					<div className="max-w-xl">
						<motion.h2
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5 }}
							className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl"
						>
							The Sleep/Wake Cycle
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-base leading-relaxed text-zinc-500"
						>
							Unlike standard cron jobs, Actors maintain their exact execution pointer and local variable state across sleeps. They don't restart from the beginning; they continue.
						</motion.p>
					</div>

					<div className="grid lg:grid-cols-2 gap-12 items-center">
						{/* Visualization */}
						<div className="relative h-80 rounded-lg border border-white/10 bg-black flex flex-col items-center justify-center overflow-hidden p-8">

							{/* Timeline Container */}
							<div className="relative w-full z-10">
								<div className="flex items-start justify-between w-full">
									{/* Node 1: Start */}
									<div className="flex flex-col items-center gap-3 z-20 w-16">
										<div
											className={`w-12 h-12 rounded-full border-2 ${activeDay === 1 ? "border-[#FF4500] bg-[#FF4500]/20 text-[#FF4500]" : "border-zinc-700 bg-zinc-900 text-zinc-500"} flex items-center justify-center transition-colors duration-500`}
										>
											<Zap className="w-5 h-5" />
										</div>
										<span className="text-xs font-mono text-zinc-400">Start</span>
									</div>

									{/* Spacer 1: Sleep */}
									<div className="flex-1 flex flex-col items-center relative px-4">
										<div className="absolute top-6 left-0 right-0 h-[2px] bg-zinc-800 -translate-y-1/2" />
										<div className={`absolute top-6 left-0 right-0 h-[2px] bg-[#FF4500]/50 -translate-y-1/2 transition-all duration-1000 origin-left ${activeDay >= 2 ? "scale-x-100" : "scale-x-0"}`} />
										<div className={`mb-8 text-[10px] font-mono uppercase tracking-widest ${activeDay === 2 ? "text-[#FF4500]" : "text-zinc-600"} transition-colors`}>Hibernating</div>
									</div>

									{/* Node 2: Resume */}
									<div className="flex flex-col items-center gap-3 z-20 w-16">
										<div
											className={`w-12 h-12 rounded-full border-2 ${activeDay === 3 ? "border-[#FF4500] bg-[#FF4500]/20 text-[#FF4500]" : "border-zinc-700 bg-zinc-900 text-zinc-500"} flex items-center justify-center transition-colors duration-500`}
										>
											<Bell className="w-5 h-5" />
										</div>
										<span className="text-xs font-mono text-zinc-400">Resume</span>
									</div>

									{/* Spacer 2: Short */}
									<div className="w-16 flex flex-col items-center relative mx-2">
										<div className="absolute top-6 left-0 right-0 h-[2px] bg-zinc-800 -translate-y-1/2" />
										<div className={`absolute top-6 left-0 right-0 h-[2px] bg-[#FF4500]/50 -translate-y-1/2 transition-all duration-500 origin-left ${activeDay >= 4 ? "scale-x-100" : "scale-x-0"}`} />
									</div>

									{/* Node 3: Done */}
									<div className="flex flex-col items-center gap-3 z-20 w-16">
										<div
											className={`w-12 h-12 rounded-full border-2 ${activeDay === 4 ? "border-[#FF4500] bg-[#FF4500]/20 text-[#FF4500]" : "border-zinc-700 bg-zinc-900 text-zinc-500"} flex items-center justify-center transition-colors duration-500`}
										>
											<Check className="w-5 h-5" />
										</div>
										<span className="text-xs font-mono text-zinc-500">Done</span>
									</div>
								</div>
							</div>

							{/* Console Log Simulation */}
							<div className="mt-12 bg-zinc-950 border border-white/10 rounded-lg p-3 font-mono text-xs text-zinc-400 w-full max-w-md z-20">
								<div className="flex items-center gap-1.5 mb-2 border-b border-white/5 pb-2">
									<div className="w-2 h-2 rounded-full bg-zinc-700" />
									<div className="w-2 h-2 rounded-full bg-zinc-700" />
									<div className="w-2 h-2 rounded-full bg-zinc-700" />
									<span className="ml-auto text-zinc-600 text-[10px]">workflow_logs.txt</span>
								</div>
								<div className="space-y-1 h-20 overflow-hidden">
									<div className={`${activeDay >= 1 ? "opacity-100" : "opacity-20"} transition-opacity`}>
										<span className="text-zinc-500">[10:00:00]</span> <span className="text-zinc-300">INFO: Workflow started. sending_email...</span>
									</div>
									<div className={`${activeDay >= 2 ? "opacity-100" : "opacity-20"} transition-opacity`}>
										<span className="text-zinc-500">[10:00:01]</span> <span className="text-zinc-400">SLEEP: Hibernating for 3 days...</span>
									</div>
									<div className={`${activeDay >= 3 ? "opacity-100" : "opacity-20"} transition-opacity`}>
										<span className="text-zinc-500">[+3d 00:00]</span> <span className="text-[#FF4500]">WAKE: Context restored from disk.</span>
									</div>
									<div className={`${activeDay >= 4 ? "opacity-100" : "opacity-20"} transition-opacity`}>
										<span className="text-zinc-500">[+3d 00:01]</span> <span className="text-zinc-300">SUCCESS: User logged in. Completing.</span>
									</div>
								</div>
							</div>
						</div>

						{/* Feature List */}
						<div className="space-y-8">
							<motion.div
								initial={{ opacity: 0, x: 20 }}
								whileInView={{ opacity: 1, x: 0 }}
								viewport={{ once: true }}
								transition={{ duration: 0.5 }}
								className="border-t border-white/10 pt-6"
							>
								<div className="mb-3 text-zinc-500">
									<Database className="h-4 w-4" />
								</div>
								<h3 className="mb-1 text-sm font-normal text-white">Implicit State</h3>
								<p className="text-sm leading-relaxed text-zinc-500">
									Forget <code className="font-mono text-xs bg-zinc-900 px-1 py-0.5 rounded">UPDATE users SET status = 'emailed'</code>. Just define a variable in your code. Rivet persists the entire JS closure automatically.
								</p>
							</motion.div>
							<motion.div
								initial={{ opacity: 0, x: 20 }}
								whileInView={{ opacity: 1, x: 0 }}
								viewport={{ once: true }}
								transition={{ duration: 0.5, delay: 0.1 }}
								className="border-t border-white/10 pt-6"
							>
								<div className="mb-3 text-zinc-500">
									<Clock className="h-4 w-4" />
								</div>
								<h3 className="mb-1 text-sm font-normal text-white">Zero-Cost Waiting</h3>
								<p className="text-sm leading-relaxed text-zinc-500">
									When you <code className="font-mono text-xs bg-zinc-900 px-1 py-0.5 rounded">await sleep('1y')</code>, the Actor serializes to disk. You pay absolutely nothing for compute while it waits.
								</p>
							</motion.div>
							<motion.div
								initial={{ opacity: 0, x: 20 }}
								whileInView={{ opacity: 1, x: 0 }}
								viewport={{ once: true }}
								transition={{ duration: 0.5, delay: 0.2 }}
								className="border-t border-white/10 pt-6"
							>
								<div className="mb-3 text-zinc-500">
									<RefreshCw className="h-4 w-4" />
								</div>
								<h3 className="mb-1 text-sm font-normal text-white">Reliability Guarantees</h3>
								<p className="text-sm leading-relaxed text-zinc-500">
									If the server crashes or deploys during a sleep, the Actor wakes up on a healthy node as if nothing happened.
								</p>
							</motion.div>
						</div>
					</div>
				</div>
			</div>
		</section>
	);
};

const WorkflowFeatures = () => {
	const features = [
		{ title: "Durable Timers", description: "Schedule code to run in the future using natural language. '2 days', 'next friday', or specific ISO dates.", icon: Calendar },
		{ title: "Human-in-the-Loop", description: "Pause execution until an external signal is received. Perfect for approval flows or 2FA verifications.", icon: Users },
		{ title: "Scheduled Jobs (Cron)", description: "Actors can be self-waking. Create a singleton actor that wakes up every hour to perform maintenance tasks.", icon: Clock },
		{ title: "Retries & Backoff", description: "Wrap flaky API calls in robust retry logic. If the process crashes, it resumes exactly where it failed.", icon: RefreshCw },
		{ title: "Sub-Workflows", description: "Spawn child actors to handle parallel tasks. The parent actor waits for results, aggregating data cleanly.", icon: GitBranch },
		{ title: "State Inspection", description: "Debug running workflows by inspecting their memory state in real-time via the dashboard or REPL.", icon: Eye },
	];

	return (
		<section className="py-48">
			<div className="mx-auto max-w-7xl px-6">
				<div className="flex flex-col gap-12">
					<div className="max-w-xl">
						<motion.h2
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5 }}
							className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl"
						>
							Primitives for Reliability
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-base leading-relaxed text-zinc-500"
						>
							Building blocks for systems that must finish what they start.
						</motion.p>
					</div>

					<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
						{features.map((f, i) => (
							<motion.div
								key={i}
								initial={{ opacity: 0, y: 20 }}
								whileInView={{ opacity: 1, y: 0 }}
								viewport={{ once: true }}
								transition={{ duration: 0.5, delay: i * 0.1 }}
								className="border-t border-white/10 pt-6"
							>
								<div className="mb-3 text-zinc-500">
									<f.icon className="h-4 w-4" />
								</div>
								<h3 className="mb-1 text-sm font-normal text-white">{f.title}</h3>
								<p className="text-sm leading-relaxed text-zinc-500">{f.description}</p>
							</motion.div>
						))}
					</div>
				</div>
			</div>
		</section>
	);
};

const UseCases = () => (
	<section className="border-t border-white/10 py-48">
		<div className="mx-auto max-w-7xl px-6">
			<div className="grid md:grid-cols-2 gap-16 items-center">
				<div>
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl"
					>
						Payment Dunning
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="mb-8 text-base leading-relaxed text-zinc-500"
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
								className="flex items-start gap-3 text-sm text-zinc-300"
							>
								<div className="mt-0.5 flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-full border border-white/10 bg-white/5">
									<Check className="h-3 w-3 text-[#FF4500]" />
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
				>
					<div className="rounded-lg border border-white/10 bg-black p-6">
						<div className="flex items-center justify-between mb-6 border-b border-white/5 pb-4">
							<div className="flex items-center gap-3">
								<div className="flex h-8 w-8 items-center justify-center rounded-lg border border-white/10 bg-white/5">
									<CreditCard className="h-4 w-4 text-white" />
								</div>
								<div>
									<div className="text-sm font-normal text-white">Invoice #INV-2049</div>
									<div className="text-xs text-zinc-500">Status: Retrying (Attempt 2/3)</div>
								</div>
							</div>
							<div className="rounded-full border border-[#FF4500]/20 bg-[#FF4500]/10 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider text-[#FF4500]">Pending</div>
						</div>
						<div className="space-y-4 text-sm font-mono">
							<div className="flex justify-between items-center text-zinc-500 text-xs">
								<span>Today</span>
								<span>Next Retry: 2d</span>
							</div>
							<div className="w-full bg-zinc-800 rounded-full h-1.5">
								<div className="bg-[#FF4500] h-1.5 rounded-full" style={{ width: "66%" }} />
							</div>
							<div className="p-3 rounded border border-white/5 bg-zinc-950 text-zinc-400">Card declined. Email sent to user@example.com. Sleeping...</div>
						</div>
					</div>
				</motion.div>
			</div>
		</div>
	</section>
);

const Ecosystem = () => (
	<section className="border-t border-white/10 py-48">
		<div className="mx-auto max-w-7xl px-6">
			<div className="flex flex-col gap-12">
				<div className="max-w-xl">
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl"
					>
						Connects with
					</motion.h2>
				</div>
				<div className="flex flex-wrap gap-2">
					{["Stripe", "Resend", "Twilio", "Slack", "Linear", "Postgres", "Discord"].map((tech, i) => (
						<motion.div
							key={tech}
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: i * 0.1 }}
							className="rounded-md border border-white/5 px-2 py-1 text-xs text-zinc-400 transition-colors hover:border-white/20 hover:text-white"
						>
							{tech}
						</motion.div>
					))}
				</div>
			</div>
		</div>
	</section>
);

export default function WorkflowsPage() {
	return (
		<div className="min-h-screen bg-black font-sans text-zinc-300 selection:bg-[#FF4500]/30 selection:text-orange-200">
			<main>
				<Hero />
				<WorkflowArchitecture />
				<WorkflowFeatures />
				<UseCases />
				<Ecosystem />

				{/* CTA Section */}
				<section className="border-t border-white/10 py-48">
					<div className="mx-auto max-w-3xl px-6 text-center">
						<motion.h2
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5 }}
							className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl"
						>
							Sleep well at night.
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="mx-auto mb-8 max-w-lg text-base leading-relaxed text-zinc-500"
						>
							Trust your critical background processes to a runtime built for durability.
						</motion.p>
						<motion.div
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.2 }}
							className="flex flex-col items-center justify-center gap-3 sm:flex-row"
						>
							<a href="/docs" className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200">
								Start Building
								<ArrowRight className="h-4 w-4" />
							</a>
							<a href="/docs/actors" className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white">
								Read the Docs
							</a>
						</motion.div>
					</div>
				</section>
			</main>
		</div>
	);
}

