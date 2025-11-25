"use client";

import React, { useState, useEffect } from "react";
import {
	Terminal,
	Zap,
	Globe,
	Github,
	ArrowRight,
	Box,
	Database,
	Layers,
	Check,
	Copy,
	Cpu,
	Server,
	RefreshCw,
	Clock,
	Shield,
	Cloud,
	Download,
	LayoutGrid,
	Activity,
	Wifi,
	Moon,
	AlertCircle,
	Gamepad2,
	MessageSquare,
	Bot,
	Users,
	FileText,
	Workflow,
	Gauge,
	Eye,
	Play,
	Brain,
	Sparkles,
	Network,
} from "lucide-react";
import { motion } from "framer-motion";
import { ScrollObserver } from "@/components/ScrollObserver";

// --- Shared Design Components ---

const Badge = ({ text, color = "orange" }) => {
	const colorClasses = {
		orange: "text-[#FF4500] border-[#FF4500]/20 bg-[#FF4500]/10",
		blue: "text-blue-400 border-blue-500/20 bg-blue-500/10",
		purple: "text-purple-400 border-purple-500/20 bg-purple-500/10",
	};

	return (
		<div
			className={`inline-flex items-center gap-2 px-3 py-1 rounded-full border text-xs font-medium mb-8 transition-colors cursor-default ${colorClasses[color]}`}
		>
			<span className={`w-1.5 h-1.5 rounded-full bg-[#FF4500] animate-pulse`} />
			{text}
		</div>
	);
};

const CodeBlock = ({ code, fileName = "agent.ts" }) => {
	const [copied, setCopied] = useState(false);

	const handleCopy = () => {
		const textArea = document.createElement("textarea");
		textArea.value = code;
		document.body.appendChild(textArea);
		textArea.select();
		try {
			document.execCommand("copy");
		} catch (err) {
			if (navigator.clipboard) {
				navigator.clipboard.writeText(code).catch((e) => console.error(e));
			}
		}
		document.body.removeChild(textArea);
		setCopied(true);
		setTimeout(() => setCopied(false), 2000);
	};

	return (
		<div className="relative group rounded-xl overflow-hidden border border-white/10 bg-zinc-900/50 backdrop-blur-xl shadow-2xl">
			<div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-white/30 to-transparent z-10" />
			<div className="flex items-center justify-between px-4 py-3 border-b border-white/5 bg-white/5">
				<div className="flex items-center gap-2">
					<div className="w-3 h-3 rounded-full bg-zinc-500/20 border border-zinc-500/50" />
					<div className="w-3 h-3 rounded-full bg-zinc-500/20 border border-zinc-500/50" />
					<div className="w-3 h-3 rounded-full bg-zinc-500/20 border border-zinc-500/50" />
				</div>
				<div className="text-xs text-zinc-500 font-mono">{fileName}</div>
				<button
					onClick={handleCopy}
					className="text-zinc-500 hover:text-white transition-colors"
				>
					{copied ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
				</button>
			</div>
			<div className="p-4 overflow-x-auto scrollbar-hide">
				<pre className="text-sm font-mono leading-relaxed text-zinc-300">
					<code>{code}</code>
				</pre>
			</div>
		</div>
	);
};

// --- Refined Agent Card with Soft Glow & Masked Corners ---
const SolutionCard = ({ title, description, icon: Icon, color = "orange" }) => {
	const colorClasses = {
		orange: {
			iconBg: "bg-[#FF4500]/10 text-[#FF4500] group-hover:bg-[#FF4500]/20",
		},
		blue: {
			iconBg: "bg-blue-500/10 text-blue-500 group-hover:bg-blue-500/20",
		},
		purple: {
			iconBg: "bg-purple-500/10 text-purple-500 group-hover:bg-purple-500/20",
		},
		emerald: {
			iconBg: "bg-emerald-500/10 text-emerald-500 group-hover:bg-emerald-500/20",
		},
	};

	const colors = colorClasses[color] || colorClasses.orange;

	return (
		<div className="group relative overflow-hidden rounded-3xl border border-white/10 bg-white/[0.02] hover:bg-white/[0.04] backdrop-blur-sm transition-all duration-500 hover:border-white/20 flex flex-col h-full p-6 hover:shadow-[0_0_50px_-12px_rgba(255,255,255,0.1)]">
			{/* Top Shine Highlight */}
			<div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-white/40 to-transparent z-10 opacity-70 group-hover:opacity-100 transition-opacity" />

			{/* Soft Glow (Gradient) */}
			<div
				className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none"
				style={{
					background: `radial-gradient(circle at top left, ${color === "orange" ? "rgba(255, 69, 0, 0.1)" : color === "blue" ? "rgba(59, 130, 246, 0.1)" : color === "purple" ? "rgba(168, 85, 247, 0.1)" : "rgba(16, 185, 129, 0.1)"}, transparent)`,
				}}
			/>

			<div className="flex items-center gap-3 mb-4 relative z-10">
				<div className={`p-2 rounded ${colors.iconBg} transition-colors duration-500`}>
					<Icon className="w-5 h-5" />
				</div>
				<h3 className="text-lg font-medium text-white tracking-tight">{title}</h3>
			</div>
			<p className="text-sm text-zinc-400 leading-relaxed relative z-10 flex-grow">
				{description}
			</p>
		</div>
	);
};

// --- Page Sections ---

const Hero = () => (
	<section className="relative pt-32 pb-20 md:pt-48 md:pb-32 overflow-hidden">
		<div className="absolute top-0 left-1/2 -translate-x-1/2 w-[1000px] h-[500px] bg-white/[0.02] blur-[100px] rounded-full pointer-events-none" />

		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-white/10 bg-white/5 text-xs font-medium text-zinc-400 mb-8 hover:border-white/20 transition-colors cursor-default"
					>
						<span className="w-2 h-2 rounded-full bg-[#FF4500] animate-pulse" />
						Rivet for Agents
					</motion.div>

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-5xl md:text-7xl font-medium text-white tracking-tighter leading-[1.1] mb-6"
					>
						The Stateful Runtime for <br />
						<span className="text-transparent bg-clip-text bg-gradient-to-b from-zinc-200 to-zinc-500">
							AI Agents.
						</span>
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="text-lg md:text-xl text-zinc-400 leading-relaxed mb-8 max-w-lg"
					>
						LLMs are stateless. Agents shouldn't be. Rivet Actors provide the persistent
						memory, tool execution environment, and long-running context your agents need to
						thrive.
					</motion.p>

					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.3 }}
						className="flex flex-col sm:flex-row items-center gap-4"
					>
						<button className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black shadow-sm hover:bg-zinc-200 transition-colors gap-2">
							Deploy Agent
							<ArrowRight className="w-4 h-4" />
						</button>
						<button className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white shadow-sm hover:border-white/20 transition-colors gap-2">
							<Play className="w-4 h-4" />
							Watch Demo
						</button>
					</motion.div>
				</div>

				<div className="flex-1 w-full max-w-xl">
					<motion.div
						initial={{ opacity: 0, scale: 0.95 }}
						animate={{ opacity: 1, scale: 1 }}
						transition={{ duration: 0.7, delay: 0.4, ease: [0.21, 0.47, 0.32, 0.98] }}
						className="relative"
					>
						<div className="absolute -inset-1 bg-gradient-to-r from-zinc-700 to-zinc-800 rounded-xl blur opacity-20" />
						<CodeBlock
							fileName="support_agent.ts"
							code={`import { actor } from "rivetkit";
import { openai } from "@ai-sdk/openai";

export const supportAgent = actor({
  // Context window persists in memory
  state: { history: [], ticketId: null },

  actions: {
    chat: async (c, message) => {
      c.state.history.push({ role: 'user', content: message });
      
      // Stream thought process
      const result = await streamText({
        model: openai('gpt-4-turbo'),
        messages: c.state.history,
        onChunk: (chunk) => c.broadcast("delta", chunk)
      });

      // Automatically save context
      c.state.history.push({ role: 'assistant', content: result.text });
      return result.text;
    }
  }
});`}
						/>
					</motion.div>
				</div>
			</div>
		</div>
	</section>
);

const MemoryArchitecture = () => {
	// Animation Step State
	const [step, setStep] = useState(0);

	useEffect(() => {
		const interval = setInterval(() => setStep((s) => (s + 1) % 3), 2500);
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
						className="text-3xl md:text-4xl font-medium text-white mb-6 tracking-tight"
					>
						Why Actors for Agents?
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-zinc-400 max-w-2xl"
					>
						Traditional architectures force you to fetch conversation history from a database
						for every single token generated. Rivet Actors keep the context hot in RAM.
					</motion.p>
				</div>

				<div className="grid lg:grid-cols-2 gap-12 items-center">
					{/* Diagram */}
					<div className="relative h-80 rounded-2xl border border-white/10 bg-zinc-900/20 flex items-center justify-center overflow-hidden">
						{/* The Loop */}
						<div className="relative z-10 flex items-center gap-8">
							{/* User */}
							<div
								className={`flex flex-col items-center transition-opacity duration-500 ${step === 0 ? "opacity-100" : "opacity-40"}`}
							>
								<div className="w-12 h-12 rounded-full bg-white flex items-center justify-center mb-2 shadow-[0_0_20px_white]">
									<Users className="w-6 h-6 text-black" />
								</div>
								<span className="text-xs font-mono text-zinc-400">User</span>
							</div>

							{/* Flow Arrow */}
							<div className="w-24 h-[2px] bg-zinc-700 relative overflow-hidden">
								<div
									className={`absolute inset-0 bg-[#FF4500] transition-transform duration-1000 ${step === 0 ? "translate-x-0" : "translate-x-full"}`}
								/>
							</div>

							{/* The Actor (Memory + Compute) */}
							<div
								className={`relative w-48 h-48 rounded-full border-2 ${step === 1 ? "border-[#FF4500] shadow-[0_0_30px_rgba(255,69,0,0.3)]" : "border-zinc-700"} bg-black flex flex-col items-center justify-center transition-all duration-500`}
							>
								<div className="absolute inset-2 rounded-full border border-white/5 border-dashed animate-[spin_10s_linear_infinite]" />
								<Brain
									className={`w-10 h-10 mb-2 transition-colors ${step === 1 ? "text-[#FF4500]" : "text-zinc-600"}`}
								/>
								<div className="text-xs font-mono text-zinc-400 text-center">
									Agent Actor
									<br />
									<span className="text-[10px] text-zinc-600">(Hot Context)</span>
								</div>

								{/* Tool Call Bubble */}
								<div
									className={`absolute -top-4 right-0 bg-zinc-800 border border-zinc-600 px-2 py-1 rounded text-[10px] text-[#FF4500] font-mono transition-all duration-300 ${step === 1 ? "opacity-100 translate-y-0" : "opacity-0 translate-y-2"}`}
								>
									Thinking...
								</div>
							</div>

							{/* Flow Arrow */}
							<div className="w-24 h-[2px] bg-zinc-700 relative overflow-hidden">
								<div
									className={`absolute inset-0 bg-[#FF4500] transition-transform duration-1000 ${step === 2 ? "translate-x-0" : "-translate-x-full"}`}
								/>
							</div>

							{/* Tool/Output */}
							<div
								className={`flex flex-col items-center transition-opacity duration-500 ${step === 2 ? "opacity-100" : "opacity-40"}`}
							>
								<div className="w-12 h-12 rounded-full border border-white/20 bg-zinc-900 flex items-center justify-center mb-2">
									<Terminal className="w-5 h-5 text-white" />
								</div>
								<span className="text-xs font-mono text-zinc-400">Tool</span>
							</div>
						</div>
					</div>

					{/* Feature List */}
					<div className="space-y-6">
						<div className="group">
							<h3 className="text-xl font-medium text-white mb-2 flex items-center gap-2">
								<Database className="w-5 h-5 text-[#FF4500]" />
								Zero-Latency Context
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								Conversation history and embedding vectors stay in the Actor's heap. No
								database queries required to "rehydrate" the agent state for each message.
							</p>
						</div>
						<div className="w-full h-[1px] bg-white/5" />
						<div className="group">
							<h3 className="text-xl font-medium text-white mb-2 flex items-center gap-2">
								<Clock className="w-5 h-5 text-blue-400" />
								Long-Running "Thought" Loops
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								Agents often need to chain multiple tool calls (CoT). Actors can run for
								minutes or hours, maintaining their state throughout the entire reasoning
								chain without timeouts.
							</p>
						</div>
						<div className="w-full h-[1px] bg-white/5" />
						<div className="group">
							<h3 className="text-xl font-medium text-white mb-2 flex items-center gap-2">
								<Users className="w-5 h-5 text-[#FF4500]" />
								Multi-User Collaboration
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								Since the Actor is a live process, multiple users can connect to the same
								Agent instance simultaneously via WebSockets to collaborate or monitor
								execution.
							</p>
						</div>
					</div>
				</div>
			</div>
		</section>
	);
};

const AgentCapabilities = () => {
	const capabilities = [
		{
			title: "Streaming by Default",
			description:
				"Native support for SSE and WebSocket streaming. Pipe tokens from the LLM directly to the client with zero overhead.",
			icon: Wifi,
			color: "blue",
		},
		{
			title: "Tool Execution Sandbox",
			description:
				"Actors provide a secure isolation boundary. Run Python scripts or API calls without blocking your main API fleet.",
			icon: Terminal,
			color: "orange",
		},
		{
			title: "Human-in-the-Loop",
			description:
				"Pause execution, wait for a human approval signal via RPC, and then resume the agent's context exactly where it left off.",
			icon: Users,
			color: "purple",
		},
		{
			title: "Scheduled Wake-ups",
			description:
				"Set an alarm for your agent. It can sleep to disk and wake up in 2 days to follow up with a user automatically.",
			icon: Clock,
			color: "emerald",
		},
		{
			title: "Knowledge Graph State",
			description:
				"Keep a complex graph of entities in memory. Update relationships dynamically as the conversation progresses.",
			icon: Network,
			color: "blue",
		},
		{
			title: "Vendor Neutral",
			description:
				"Swap between OpenAI, Anthropic, or local Llama models instantly. The Actor pattern abstracts the underlying intelligence.",
			icon: Globe,
			color: "orange",
		},
	];

	return (
		<section className="py-32 bg-zinc-900/20 relative">
			<div className="max-w-7xl mx-auto px-6">
				<div className="mb-20 text-center">
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						Built for the Agentic Future
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-zinc-400"
					>
						The infrastructure primitives you need to move beyond simple chatbots.
					</motion.p>
				</div>

				<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
					{capabilities.map((cap, idx) => (
						<motion.div
							key={idx}
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: idx * 0.1 }}
						>
							<SolutionCard {...cap} />
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
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
					>
						<div className="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-white/10 bg-white/5 text-xs font-medium text-zinc-400 mb-8 hover:border-white/20 transition-colors cursor-default">
							<span className="w-2 h-2 rounded-full bg-blue-400 animate-pulse" />
							Case Study
						</div>
						<h2 className="text-3xl font-medium text-white mb-6 tracking-tight">
							Customer Support Swarms
						</h2>
						<p className="text-lg text-zinc-400 mb-8">
							Deploy a dedicated Actor for every single active support ticket.
						</p>
						<ul className="space-y-4">
							{[
								"Isolation: One crashed agent doesn't affect others",
								"Context: Full ticket history in memory (up to 128k tokens)",
								"Handoff: Transfer actor state to a human instantly",
							].map((item, i) => (
								<li key={i} className="flex items-center gap-3 text-zinc-300">
									<div className="w-5 h-5 rounded-full bg-blue-500/20 flex items-center justify-center">
										<Check className="w-3 h-3 text-blue-400" />
									</div>
									{item}
								</li>
							))}
						</ul>
					</motion.div>
				</div>
				<motion.div
					initial={{ opacity: 0, scale: 0.95 }}
					whileInView={{ opacity: 1, scale: 1 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.2 }}
					className="relative"
				>
					<div className="absolute inset-0 bg-gradient-to-r from-blue-500/20 to-transparent rounded-2xl blur-2xl" />
					<div className="relative rounded-2xl border border-white/10 bg-zinc-900/50 backdrop-blur-sm p-6 shadow-2xl">
						<div className="flex items-center justify-between mb-6 border-b border-white/5 pb-4">
							<div className="flex items-center gap-3">
								<div className="w-8 h-8 rounded bg-blue-500/20 flex items-center justify-center">
									<Bot className="w-5 h-5 text-blue-400" />
								</div>
								<div>
									<div className="text-sm font-medium text-white">Ticket #9402</div>
									<div className="text-xs text-zinc-500">Agent: Active</div>
								</div>
							</div>
							<div className="px-2 py-1 rounded bg-green-500/10 text-green-400 text-xs border border-green-500/20">
								Online
							</div>
						</div>
						<div className="space-y-4 text-sm font-mono">
							<div className="p-3 rounded bg-zinc-950 border border-white/5 text-zinc-400">
								Analysing user request... accessing database...
							</div>
							<div className="p-3 rounded bg-blue-900/20 border border-blue-500/30 text-blue-200">
								Found 3 matching orders. Asking clarification...
							</div>
							<div className="flex gap-2">
								<div className="h-2 w-2 rounded-full bg-blue-500 animate-bounce" />
								<div
									className="h-2 w-2 rounded-full bg-blue-500 animate-bounce"
									style={{ animationDelay: "0.1s" }}
								/>
								<div
									className="h-2 w-2 rounded-full bg-blue-500 animate-bounce"
									style={{ animationDelay: "0.2s" }}
								/>
							</div>
						</div>
					</div>
				</motion.div>
			</div>
		</div>
	</section>
);

const OrchestrationSection = () => {
	return (
		<section className="py-24 bg-zinc-900/20 border-t border-white/5 relative overflow-hidden">
			<div className="max-w-7xl mx-auto px-6">
				<div className="flex flex-col lg:flex-row gap-16 items-center">
					<div className="flex-1 max-w-xl order-2 lg:order-1">
						<div className="relative">
							<div className="absolute -inset-1 bg-gradient-to-br from-[#FF4500]/20 to-purple-500/20 rounded-xl blur-xl opacity-40" />
							<CodeBlock
								fileName="orchestrator.ts"
								code={`import { actor } from "rivetkit";

export const manager = actor({
  state: { subtasks: [] },

  actions: {
    runWorkflow: async (c, goal) => {
      // 1. Spawn specialized workers
      const researcher = await c.spawn(researchAgent);
      const coder = await c.spawn(codingAgent);

      // 2. Parallel execution (RPC)
      const [context, code] = await Promise.all([
         researcher.rpc.gatherInfo(goal),
         coder.rpc.generate(goal)
      ]);

      // 3. Synthesize results
      return { context, code, status: 'complete' };
    }
  }
});`}
							/>
						</div>
					</div>

					<div className="flex-1 order-1 lg:order-2">
						<motion.div
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5 }}
						>
							<div className="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-white/10 bg-white/5 text-xs font-medium text-zinc-400 mb-8 hover:border-white/20 transition-colors cursor-default">
								<span className="w-2 h-2 rounded-full bg-[#FF4500] animate-pulse" />
								Multi-Agent Systems
							</div>
							<h2 className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight">
								Orchestrate <br />
								<span className="text-[#FF4500]">Agent Swarms.</span>
							</h2>
							<p className="text-lg text-zinc-400 leading-relaxed mb-8">
								Don't stop at one. Build hierarchical trees of Actors where managers delegate
								tasks to specialized workers. Rivet handles the supervision, networking, and
								message passing between them automatically.
							</p>

							<div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
								{[
									{ title: "Parallel Processing", desc: "Run 100 research agents at once." },
									{
										title: "Supervision Trees",
										desc: "Restart workers if they hallucinate or crash.",
									},
									{
										title: "Shared State",
										desc: "Pass context references instantly between actors.",
									},
									{
										title: "Event Bus",
										desc: "Pub/Sub messaging between swarm members.",
									},
								].map((item, i) => (
									<div
										key={i}
										className="p-4 rounded-xl border border-white/10 bg-white/[0.02] hover:bg-white/[0.04] backdrop-blur-sm transition-all hover:border-white/20"
									>
										<h4 className="text-white font-medium text-sm mb-1">{item.title}</h4>
										<p className="text-zinc-500 text-xs">{item.desc}</p>
									</div>
								))}
							</div>
						</motion.div>
					</div>
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
				className="text-3xl font-medium text-white mb-12 tracking-tight"
			>
				Works with your stack
			</motion.h2>
			<div className="flex flex-wrap justify-center gap-4">
				{["LangChain", "LlamaIndex", "Vercel AI SDK", "OpenAI", "Anthropic", "HuggingFace"].map(
					(tech) => (
						<div
							key={tech}
							className="px-6 py-3 rounded-xl border border-white/10 bg-black/50 text-zinc-400 text-sm font-mono hover:text-white hover:border-white/30 transition-colors cursor-default backdrop-blur-sm"
						>
							{tech}
						</div>
					),
				)}
			</div>
		</div>
	</section>
);

export default function AgentPage() {
	return (
		<ScrollObserver>
			<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-[#FF4500]/30 selection:text-orange-200">
				<main>
					<Hero />
					<MemoryArchitecture />
					<AgentCapabilities />
					<UseCases />
					<OrchestrationSection />
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
								Stop building generic bots.
							</motion.h2>
							<motion.p
								initial={{ opacity: 0, y: 20 }}
								whileInView={{ opacity: 1, y: 0 }}
								viewport={{ once: true }}
								transition={{ duration: 0.5, delay: 0.1 }}
								className="text-lg text-zinc-400 mb-10"
							>
								Start building stateful, durable, intelligent agents that actually work.
							</motion.p>
							<motion.div
								initial={{ opacity: 0, y: 20 }}
								whileInView={{ opacity: 1, y: 0 }}
								viewport={{ once: true }}
								transition={{ duration: 0.5, delay: 0.2 }}
								className="flex flex-col sm:flex-row items-center justify-center gap-4"
							>
								<button className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black shadow-sm hover:bg-zinc-200 transition-colors gap-2">
									Start Building Now
								</button>
								<button className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white shadow-sm hover:border-white/20 transition-colors gap-2">
									Read the Docs
								</button>
							</motion.div>
						</div>
					</section>
				</main>
			</div>
		</ScrollObserver>
	);
}

