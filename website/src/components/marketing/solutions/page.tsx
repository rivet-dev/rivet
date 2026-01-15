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
} from "lucide-react";
import { motion } from "framer-motion";

// --- Shared Design Components ---
const Badge = ({ text, color = "orange" }) => {
	const colorClasses = {
		orange: "text-orange-400 border-orange-500/20 bg-orange-500/10",
		blue: "text-blue-400 border-blue-500/20 bg-blue-500/10",
		purple: "text-purple-400 border-purple-500/20 bg-purple-500/10",
	};

	return (
		<div
			className={`inline-flex items-center gap-2 px-3 py-1 rounded-full border text-xs font-medium mb-8 transition-colors cursor-default ${colorClasses[color]}`}
		>
			<span className={`w-1.5 h-1.5 rounded-full ${color === "orange" ? "bg-orange-400" : color === "blue" ? "bg-blue-400" : "bg-purple-400"} animate-pulse`} />
			{text}
		</div>
	);
};

const CodeBlock = ({ code, fileName = "agent.ts" }) => {
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
											if (["import", "from", "export", "const", "return", "async", "await", "function", "let", "var"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-purple-400">{part}</span>);
											}
											// Functions & Special Rivet Terms
											else if (["actor", "broadcast", "streamText", "spawn", "rpc"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-400">{part}</span>);
											}
											// Object Keys / Properties / Methods
											else if (["state", "actions", "history", "ticketId", "chat", "message", "model", "messages", "onChunk", "delta", "role", "content", "text", "subtasks", "runWorkflow", "goal", "researcher", "coder", "gatherInfo", "generate", "context", "code", "status", "complete"].includes(trimmed)) {
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

// --- Refined Agent Card matching landing page style with color highlights ---
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
		<div className="absolute top-0 left-1/2 -translate-x-1/2 w-[1000px] h-[500px] bg-purple-500/[0.03] blur-[100px] rounded-full pointer-events-none" />

		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<Badge text="Rivet for Agents" color="purple" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="text-5xl md:text-7xl font-medium text-white tracking-tight leading-[1.1] mb-6"
					>
						Build <br />
						<span className="text-purple-400">AI Agents.</span>
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg md:text-xl text-zinc-400 leading-relaxed mb-8 max-w-lg"
					>
						LLMs are stateless. Agents shouldn't be. Rivet Actors provide the persistent memory, tool execution environment, and long-running context your agents need to thrive.
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
					</motion.div>
				</div>
				<div className="flex-1 w-full max-w-xl">
					<div className="relative">
						<div className="absolute -inset-1 bg-gradient-to-r from-purple-500/20 to-blue-500/20 rounded-xl blur opacity-40" />
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
					</div>
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
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						Why Actors for Agents?
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-zinc-400 max-w-2xl text-lg leading-relaxed"
					>
						Traditional architectures force you to fetch conversation history from a database for every single token generated. Rivet Actors keep the context hot in RAM.
					</motion.p>
				</div>

				<div className="grid lg:grid-cols-2 gap-12 items-center">
					{/* Diagram */}
					<div className="relative h-80 rounded-2xl border border-white/10 bg-zinc-900/20 flex items-center justify-center overflow-hidden">
						{/* The Loop */}
						<div className="relative z-10 flex items-center gap-4">
							{/* User */}
							<div className={`flex flex-col items-center transition-opacity duration-500 ${step === 0 ? "opacity-100" : "opacity-40"}`}>
								<div className="w-12 h-12 rounded-full bg-white flex items-center justify-center mb-2 shadow-[0_0_20px_white]">
									<Users className="w-6 h-6 text-black" />
								</div>
								<span className="text-xs font-mono text-zinc-400">User</span>
							</div>

							{/* Flow Arrow */}
							<div className="w-16 h-[2px] bg-zinc-700 relative overflow-hidden">
								<div className={`absolute inset-0 bg-purple-500 transition-transform duration-1000 ${step === 0 ? "translate-x-0" : "translate-x-full"}`} />
							</div>

							{/* The Actor (Memory + Compute) */}
							<div
								className={`relative w-40 h-40 rounded-full border-2 ${step === 1 ? "border-purple-500 shadow-[0_0_30px_rgba(168,85,247,0.3)]" : "border-zinc-700"} bg-black flex flex-col items-center justify-center transition-all duration-500`}
							>
								<div className="absolute inset-2 rounded-full border border-white/5 border-dashed animate-[spin_10s_linear_infinite]" />
								<Brain className={`w-8 h-8 mb-2 transition-colors ${step === 1 ? "text-purple-400" : "text-zinc-600"}`} />
								<div className="text-xs font-mono text-zinc-400 text-center">
									Agent Actor
									<br />
									<span className="text-[10px] text-zinc-600">(Hot Context)</span>
								</div>

								{/* Tool Call Bubble */}
								<div
									className={`absolute -top-4 right-0 bg-zinc-800 border border-zinc-600 px-2 py-1 rounded text-[10px] text-orange-400 font-mono transition-all duration-300 ${step === 1 ? "opacity-100 translate-y-0" : "opacity-0 translate-y-2"}`}
								>
									Thinking...
								</div>
							</div>
							{/* Flow Arrow */}
							<div className="w-16 h-[2px] bg-zinc-700 relative overflow-hidden">
								<div className={`absolute inset-0 bg-purple-500 transition-transform duration-1000 ${step === 2 ? "translate-x-0" : "-translate-x-full"}`} />
							</div>
							{/* Tool/Output */}
							<div className={`flex flex-col items-center transition-opacity duration-500 ${step === 2 ? "opacity-100" : "opacity-40"}`}>
								<div className="w-12 h-12 rounded-full border border-white/20 bg-zinc-900 flex items-center justify-center mb-2">
									<Terminal className="w-5 h-5 text-white" />
								</div>
								<span className="text-xs font-mono text-zinc-400">Tool</span>
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
								<Database className="w-5 h-5 text-purple-400" />
								Zero-Latency Context
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								Conversation history and embedding vectors stay in the Actor's heap. No database queries required to "rehydrate" the agent state for each message.
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
								Long-Running "Thought" Loops
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								Agents often need to chain multiple tool calls (CoT). Actors can run for minutes or hours, maintaining their state throughout the entire reasoning chain without timeouts.
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
								<Users className="w-5 h-5 text-orange-400" />
								Multi-User Collaboration
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								Since the Actor is a live process, multiple users can connect to the same Agent instance simultaneously via WebSockets to collaborate or monitor execution.
							</p>
						</motion.div>
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
			description: "Native support for SSE and WebSocket streaming. Pipe tokens from the LLM directly to the client with zero overhead.",
			icon: Wifi,
			color: "blue",
		},
		{
			title: "Tool Execution Sandbox",
			description: "Actors provide a secure isolation boundary. Run Python scripts or API calls without blocking your main API fleet.",
			icon: Terminal,
			color: "orange",
		},
		{
			title: "Human-in-the-Loop",
			description: "Pause execution, wait for a human approval signal via RPC, and then resume the agent's context exactly where it left off.",
			icon: Users,
			color: "purple",
		},
		{
			title: "Scheduled Wake-ups",
			description: "Set an alarm for your agent. It can sleep to disk and wake up in 2 days to follow up with a user automatically.",
			icon: Clock,
			color: "emerald",
		},
		{
			title: "Knowledge Graph State",
			description: "Keep a complex graph of entities in memory. Update relationships dynamically as the conversation progresses.",
			icon: Network,
			color: "blue",
		},
		{
			title: "Vendor Neutral",
			description: "Swap between OpenAI, Anthropic, or local Llama models instantly. The Actor pattern abstracts the underlying intelligence.",
			icon: Globe,
			color: "orange",
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
					<h2 className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight">Built for the Agentic Future</h2>
					<p className="text-zinc-400 text-lg leading-relaxed">The infrastructure primitives you need to move beyond simple chatbots.</p>
				</motion.div>

				<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
					{capabilities.map((cap, idx) => (
						<motion.div
							key={idx}
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: idx * 0.05 }}
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
					<Badge text="Case Study" color="blue" />
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						Customer Support Swarms
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg text-zinc-400 mb-8 leading-relaxed"
					>
						Deploy a dedicated Actor for every single active support ticket.
					</motion.p>
					<ul className="space-y-4">
						{["Isolation: One crashed agent doesn't affect others", "Context: Full ticket history in memory (up to 128k tokens)", "Handoff: Transfer actor state to a human instantly"].map((item, i) => (
							<motion.li
								key={i}
								initial={{ opacity: 0, x: -20 }}
								whileInView={{ opacity: 1, x: 0 }}
								viewport={{ once: true }}
								transition={{ duration: 0.5, delay: 0.2 + i * 0.1 }}
								className="flex items-center gap-3 text-zinc-300"
							>
								<div className="w-5 h-5 rounded-full bg-blue-500/20 flex items-center justify-center">
									<Check className="w-3 h-3 text-blue-400" />
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
					<div className="absolute inset-0 bg-gradient-to-r from-blue-500/20 to-transparent rounded-2xl blur-2xl" />
					<div className="relative rounded-2xl border border-white/10 bg-zinc-900 p-6 shadow-2xl">
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
							<div className="px-2 py-1 rounded bg-green-500/10 text-green-400 text-xs border border-green-500/20">Online</div>
						</div>
						<div className="space-y-4 text-sm font-mono">
							<div className="p-3 rounded bg-zinc-950 border border-white/5 text-zinc-400">Analysing user request... accessing database...</div>
							<div className="p-3 rounded bg-blue-900/20 border border-blue-500/30 text-blue-200">Found 3 matching orders. Asking clarification...</div>
							<div className="flex gap-2">
								<div className="h-2 w-2 rounded-full bg-blue-500 animate-bounce" />
								<div className="h-2 w-2 rounded-full bg-blue-500 animate-bounce delay-75" />
								<div className="h-2 w-2 rounded-full bg-blue-500 animate-bounce delay-150" />
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
							<div className="absolute -inset-1 bg-gradient-to-br from-orange-500/20 to-purple-500/20 rounded-xl blur-xl opacity-40" />
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
						<Badge text="Multi-Agent Systems" color="orange" />
						<motion.h2
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5 }}
							className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
						>
							Orchestrate <br />
							<span className="text-orange-400">Agent Swarms.</span>
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-lg text-zinc-400 leading-relaxed mb-8"
						>
							Don't stop at one. Build hierarchical trees of Actors where managers delegate tasks to specialized workers. Rivet handles the supervision, networking, and message passing between them automatically.
						</motion.p>
						<div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
							{[
								{ title: "Parallel Processing", desc: "Run 100 research agents at once." },
								{ title: "Supervision Trees", desc: "Restart workers if they hallucinate or crash." },
								{ title: "Shared State", desc: "Pass context references instantly between actors." },
								{ title: "Event Bus", desc: "Pub/Sub messaging between swarm members." },
							].map((item, i) => (
								<motion.div
									key={i}
									initial={{ opacity: 0, y: 20 }}
									whileInView={{ opacity: 1, y: 0 }}
									viewport={{ once: true }}
									transition={{ duration: 0.5, delay: 0.2 + i * 0.05 }}
									className="p-4 rounded-xl border border-white/5 bg-black/40 hover:bg-white/[0.02] hover:border-white/10 transition-all"
								>
									<h4 className="text-white font-medium text-sm mb-1">{item.title}</h4>
									<p className="text-zinc-500 text-xs">{item.desc}</p>
								</motion.div>
							))}
						</div>
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
				className="text-3xl md:text-5xl font-medium text-white mb-12 tracking-tight"
			>
				Works with your stack
			</motion.h2>
			<div className="flex flex-wrap justify-center gap-4">
				{["LangChain", "LlamaIndex", "Vercel AI SDK", "OpenAI", "Anthropic", "HuggingFace"].map((tech, i) => (
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

export default function AgentsPage() {
	return (
		<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-purple-500/30 selection:text-purple-200">
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
							className="text-lg text-zinc-400 mb-10 leading-relaxed"
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
							<a href="https://dashboard.rivet.dev/" className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black shadow-sm hover:bg-zinc-200 transition-colors">
								Start Building Now
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

