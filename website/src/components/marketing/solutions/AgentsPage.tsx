"use client";

import { useState, useEffect } from "react";
import {
	Terminal,
	ArrowRight,
	Database,
	Check,
	RefreshCw,
	Clock,
	Globe,
	Users,
	Network,
	Lock,
	Split,
	ArrowLeftRight,
	GitMerge,
	Siren,
	Workflow,
	Microchip,
	BrainCircuit,
	Boxes,
	Wifi,
	FileCode,
} from "lucide-react";
import { motion } from "framer-motion";

// --- Shared Design Components ---
const Badge = ({ text, color = "purple" }: { text: string; color?: "purple" | "blue" | "orange" }) => {
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

const CodeBlock = ({ code, fileName = "agent.ts" }: { code: string; fileName?: string }) => {
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
										const tokens: JSX.Element[] = [];
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

											if (["import", "from", "export", "const", "return", "async", "await", "function", "let", "var"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-purple-400">{part}</span>);
											}
											else if (["actor", "broadcast", "streamText", "spawn", "rpc"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-400">{part}</span>);
											}
											else if (["state", "actions", "history", "goals", "role", "content", "text", "calls", "execute", "generate", "tools", "ai"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-300">{part}</span>);
											}
											else if (part.startsWith('"') || part.startsWith("'")) {
												tokens.push(<span key={j} className="text-purple-300">{part}</span>);
											}
											else if (!isNaN(Number(trimmed)) && trimmed !== "") {
												tokens.push(<span key={j} className="text-emerald-400">{part}</span>);
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

// --- Feature Card matching site style ---
const FeatureCard = ({ title, description, icon: Icon, color = "purple" }: { 
	title: string; 
	description: string; 
	icon: React.ComponentType<{ className?: string }>; 
	color?: "purple" | "blue" | "orange" | "emerald" | "zinc" 
}) => {
	const getColorClasses = (col: string) => {
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
					bg: "bg-purple-500/10",
					text: "text-purple-400",
					hoverBg: "group-hover:bg-purple-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(168,85,247,0.5)]",
					border: "border-purple-500",
					glow: "rgba(168,85,247,0.15)",
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

// --- Page Sections ---
const Hero = () => (
	<section className="relative pt-32 pb-20 md:pt-48 md:pb-32 overflow-hidden">
		<div className="absolute top-0 left-1/2 -translate-x-1/2 w-[1000px] h-[500px] bg-purple-500/[0.05] blur-[100px] rounded-full pointer-events-none" />

		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<Badge text="The Agentic Primitive" color="purple" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="text-5xl md:text-7xl font-medium text-white tracking-tight leading-[1.1] mb-6"
					>
						Agent <br />
						<span className="text-purple-400">Orchestration.</span>
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg md:text-xl text-zinc-400 leading-relaxed mb-8 max-w-lg"
					>
						LLMs are stateless. Agents need durability. Rivet Actors provide the persistent memory and orchestration logic required to build autonomous AI systems.
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
						<a href="/templates" className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white shadow-sm hover:border-white/20 transition-colors gap-2">
							View Examples
						</a>
					</motion.div>
				</div>
				<div className="flex-1 w-full max-w-xl">
					<div className="relative">
						<div className="absolute -inset-1 bg-gradient-to-r from-purple-500/20 to-blue-500/20 rounded-xl blur opacity-40" />
						<CodeBlock
							fileName="reasoning_actor.ts"
							code={`import { actor } from "rivetkit";

export const smartAgent = actor({
  // Persistent memory state
  state: { history: [], goals: [] },

  actions: {
    think: async (c, input) => {
      // Actors hold context in RAM across requests
      c.state.history.push({ role: 'user', content: input });
      
      const res = await c.ai.generate(c.state.history);
      
      // Atomic tool execution
      const toolOutput = await c.tools.execute(res.calls);
      c.state.history.push({ role: 'assistant', content: res.text });
      
      return res.text;
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

const AgentArchitecture = () => {
	return (
		<section className="py-24 bg-black border-y border-white/5 relative">
			<div className="max-w-7xl mx-auto px-6 text-center">
				<div className="mb-16">
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						Build Agents with Actors
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-zinc-400 max-w-2xl mx-auto text-lg leading-relaxed"
					>
						Actors provide long-running processes, in-memory context, and realtime/native protocols for building stateful AI agents.
					</motion.p>
				</div>

				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5, delay: 0.2 }}
					className="relative h-[500px] w-full max-w-5xl mx-auto rounded-2xl border border-white/10 bg-zinc-900/10 flex items-center justify-center overflow-hidden p-8"
				>
					<div className="absolute inset-0 bg-[linear-gradient(rgba(168,85,247,0.03)_1px,transparent_1px),linear-gradient(90deg,rgba(168,85,247,0.03)_1px,transparent_1px)] bg-[size:40px_40px]" />
					
					{/* Central Agent Actor */}
					<div className="relative z-10 w-[450px] h-[300px] rounded-2xl border border-purple-500/30 bg-zinc-950/90 backdrop-blur-xl flex flex-col p-6 shadow-[0_0_100px_rgba(168,85,247,0.1)]">
						<div className="flex items-center justify-between mb-6">
							<div className="flex items-center gap-2">
								<div className="w-8 h-8 rounded-lg bg-purple-500/20 flex items-center justify-center border border-purple-500/40">
									<BrainCircuit className="w-5 h-5 text-purple-400" />
								</div>
								<span className="text-sm font-medium text-white tracking-tight">Agent Actor: uuid-402</span>
							</div>
							<div className="px-2 py-0.5 rounded bg-green-500/10 text-green-400 text-[10px] font-medium border border-green-500/20 uppercase tracking-widest">Active</div>
						</div>

						<div className="grid grid-cols-2 gap-4 flex-1">
							<div className="rounded-xl border border-white/5 bg-white/5 p-4 flex flex-col gap-2">
								<div className="flex items-center gap-2 text-purple-300 font-mono text-[10px] uppercase tracking-wider">
									<Database className="w-3 h-3" /> State
								</div>
								<div className="flex-1 bg-black/40 rounded p-2 font-mono text-[9px] text-zinc-500 overflow-hidden text-left leading-relaxed">
									{`history: [...] \ncontext_window: 12k \nactive_goal: "refactor"`}
								</div>
							</div>
							<div className="rounded-xl border border-white/5 bg-white/5 p-4 flex flex-col gap-2">
								<div className="flex items-center gap-2 text-purple-300 font-mono text-[10px] uppercase tracking-wider">
									<Microchip className="w-3 h-3" /> Reasoning
								</div>
								<div className="flex-1 flex flex-col items-center justify-center gap-2">
									<div className="w-full h-1 bg-zinc-800 rounded-full overflow-hidden">
										<div className="h-full bg-purple-500 w-1/3 animate-[pulse_2s_infinite]" />
									</div>
									<span className="text-[9px] text-zinc-500">Planning step...</span>
								</div>
							</div>
							<div className="rounded-xl border border-white/5 bg-white/5 p-3 col-span-2 flex items-center gap-4">
								<div className="text-purple-300 font-mono text-[10px] uppercase tracking-wider flex items-center gap-2 whitespace-nowrap">
									<Terminal className="w-3 h-3" /> Active Tools:
								</div>
								<div className="flex gap-2">
									<div className="px-2 py-1 rounded bg-zinc-900 border border-zinc-700 text-[9px] text-zinc-400">GIT-WORKSPACE</div>
									<div className="px-2 py-1 rounded bg-zinc-900 border border-zinc-700 text-[9px] text-zinc-400">SHELL-EXEC</div>
								</div>
							</div>
						</div>
					</div>

					{/* Connection Arrows */}
					<div className="absolute inset-0 pointer-events-none flex items-center justify-center">
						<div className="w-full h-full relative">
							<div className="absolute top-1/2 left-[5%] w-[15%] h-[1px] bg-gradient-to-r from-transparent via-purple-500/50 to-purple-500 animate-pulse" />
							<div className="absolute top-1/2 left-[4%] -translate-y-1/2 text-[10px] font-mono text-zinc-600">CLIENT REQUEST</div>
							
							<div className="absolute top-1/2 right-[5%] w-[15%] h-[1px] bg-gradient-to-l from-transparent via-purple-500/50 to-purple-500" />
							<div className="absolute top-1/2 right-[4%] -translate-y-1/2 text-[10px] font-mono text-zinc-600 text-right">TOOL RESPONSE</div>
						</div>
					</div>
				</motion.div>
			</div>
		</section>
	);
};

const AgentFeatures = () => {
	const features = [
		{ title: "Long-Running Tasks", description: "Agents can 'hibernate' while waiting for external API callbacks or human approval, consuming zero resources.", icon: Clock, color: "zinc" as const },
		{ title: "Realtime", description: "Stream responses and updates in real-time via WebSockets. Agents can broadcast events, progress updates, and intermediate results as they work.", icon: Wifi, color: "orange" as const },
		{ title: "Durable Execution", description: "Actors persist their thought process. If the server restarts, the agent resumes exactly where it left off, mid-thought.", icon: RefreshCw, color: "purple" as const },
		{ title: "Orchestration", description: "Communicate between agents using low-latency RPC. Agents can 'hand off' state to each other effortlessly. Auto-scaling ensures optimal resource allocation as workloads grow.", icon: ArrowLeftRight, color: "purple" as const },
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
					<h2 className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight">Why Actors for Agents?</h2>
					<p className="text-zinc-400 max-w-2xl text-lg leading-relaxed">Rivet Actors are the fundamental building blocks for agents that need to reason about state over time.</p>
				</motion.div>
				<div className="grid md:grid-cols-2 gap-6">
					{features.map((f, i) => (
						<motion.div
							key={i}
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: i * 0.05 }}
						>
							<FeatureCard title={f.title} description={f.description} icon={f.icon} color={f.color} />
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
					<Badge text="Application" color="purple" />
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						Autonomous Coding Agent
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg text-zinc-400 mb-8 leading-relaxed"
					>
						Enable agents to operate directly on codebases with real-time feedback loops and durable process context.
					</motion.p>
					<ul className="space-y-4">
						{[
							"Real-time Monitoring: Stream shell outputs and linter results directly to the user interface.",
							"Durable Tool Loops: Agents can execute complex bash scripts and resume if the task is interrupted.",
							"Context Retention: Long-term memory of project structure, preventing repetitive discovery steps.",
						].map((item, i) => (
							<motion.li
								key={i}
								initial={{ opacity: 0, x: -20 }}
								whileInView={{ opacity: 1, x: 0 }}
								viewport={{ once: true }}
								transition={{ duration: 0.5, delay: 0.2 + i * 0.1 }}
								className="flex items-center gap-3 text-zinc-300"
							>
								<div className="w-5 h-5 rounded-full bg-purple-500/20 flex items-center justify-center">
									<Check className="w-3 h-3 text-purple-400" />
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
					<div className="absolute inset-0 bg-gradient-to-r from-purple-500/20 to-transparent rounded-2xl blur-2xl" />
					<div className="relative rounded-2xl border border-white/10 bg-zinc-900 p-6 shadow-2xl">
						<div className="flex items-center justify-between mb-6 border-b border-white/5 pb-4">
							<div className="flex items-center gap-3">
								<div className="w-8 h-8 rounded bg-purple-500/20 flex items-center justify-center">
									<FileCode className="w-5 h-5 text-purple-400" />
								</div>
								<div>
									<div className="text-sm font-medium text-white">Task: Fix CVE-2024-81</div>
									<div className="text-xs text-zinc-500">Sub-processes: 2</div>
								</div>
							</div>
							<div className="px-2 py-1 rounded bg-blue-500/10 text-blue-400 text-xs border border-blue-500/20 tracking-widest font-medium">MONITORING</div>
						</div>
						<div className="space-y-4 text-sm font-mono">
							<div className="p-3 rounded bg-zinc-950 border border-white/5 text-zinc-400 flex flex-col gap-2">
								<div className="flex justify-between">
									<span className="text-zinc-500 text-xs">SHELL</span>
									<span className="text-green-400 text-[10px]">REAL-TIME</span>
								</div>
								<div className="text-[10px] opacity-70">
									$ npm test --grep security-patch <br/>
									<span className="text-green-500">✓ Patch verified (12 tests passed)</span>
								</div>
							</div>
							<div className="p-3 rounded bg-zinc-950 border border-white/5 text-zinc-400 flex justify-between group cursor-default">
								<span>Linter</span>
								<span className="text-purple-400 group-hover:text-purple-300 text-[10px]">Waiting...</span>
							</div>
						</div>
					</div>
				</motion.div>
			</div>
		</div>
	</section>
);

const Ecosystem = () => (
	<section className="py-24 bg-black border-t border-white/5 relative overflow-hidden">
		<div className="max-w-7xl mx-auto px-6 relative z-10 text-center">
			<motion.h2
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5 }}
				className="text-3xl md:text-5xl font-medium text-white mb-12 tracking-tight"
			>
				Integrates with
			</motion.h2>
			<div className="flex flex-wrap justify-center gap-4">
				{["LangChain", "LlamaIndex", "Vercel AI SDK", "OpenAI", "Anthropic", "Hugging Face"].map((tech, i) => (
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
				<AgentArchitecture />
				<AgentFeatures />
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
							Build smarter agents.
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-lg text-zinc-400 mb-10 leading-relaxed"
						>
							Give your AI the stateful foundation it needs to actually solve complex problems.
						</motion.p>
						<motion.div
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.2 }}
							className="flex flex-col sm:flex-row items-center justify-center gap-4"
						>
							<a href="/docs" className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black shadow-sm hover:bg-zinc-200 transition-colors">
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
