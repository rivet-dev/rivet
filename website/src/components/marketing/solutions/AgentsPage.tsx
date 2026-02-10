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

// Simple code block matching landing page style
const CodeBlock = ({ code, fileName = "agent.ts" }: { code: string; fileName?: string }) => {
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

											if (["import", "from", "export", "const", "return", "async", "await", "function", "let", "var", "while", "true", "false"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-purple-400">{part}</span>);
											}
											else if (["actor", "generateText", "openai", "spawn", "rpc", "ai"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-400">{part}</span>);
											}
											else if (["state", "actions", "history", "goals", "role", "content", "text", "model", "messages", "think", "input", "broadcast", "push", "c", "run", "queue", "next"].includes(trimmed)) {
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
							AI Agents
						</span>
					</motion.div>

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="mb-6 text-4xl font-normal leading-[1.1] tracking-tight text-white md:text-6xl"
					>
						Agent Orchestration
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="mb-8 max-w-lg text-base leading-relaxed text-zinc-500"
					>
						LLMs are stateless. Agents need durability. Rivet Actors provide the persistent memory and orchestration logic required to build autonomous AI systems.
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
						<a href="/templates/ai-agent" className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white">
							View Example
						</a>
					</motion.div>
				</div>
				<div className="flex-1 w-full max-w-xl">
					<CodeBlock
						fileName="reasoning_actor.ts"
						code={`import { actor } from "rivetkit";
import { generateText } from "ai";
import { openai } from "@ai-sdk/openai";

export const smartAgent = actor({
  // Persistent memory state
  state: { history: [], goals: [] },

  actions: {
    think: async (c, input) => {
      // Context is held in RAM across requests
      c.state.history.push({ role: "user", content: input });

      const { text } = await generateText({
        model: openai("gpt-4o"),
        messages: c.state.history,
      });

      c.state.history.push({ role: "assistant", content: text });
      c.broadcast("response", text);
      return text;
    }
  }
});`}
					/>
				</div>
			</div>
		</div>
	</section>
);

const AgentArchitecture = () => {
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
							Build Agents with Actors
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-base leading-relaxed text-zinc-500"
						>
							Actors provide long-running processes, in-memory context, and realtime protocols for building stateful AI agents.
						</motion.p>
					</div>

					<motion.div
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="relative h-[400px] w-full rounded-lg border border-white/10 bg-black flex items-center justify-center overflow-hidden p-8"
					>
						<div className="absolute inset-0 bg-[linear-gradient(rgba(255,255,255,0.02)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.02)_1px,transparent_1px)] bg-[size:40px_40px]" />

						{/* Central Agent Actor */}
						<div className="relative z-10 w-[450px] h-[280px] rounded-lg border border-white/10 bg-zinc-950 flex flex-col p-6">
							<div className="flex items-center justify-between mb-6">
								<div className="flex items-center gap-2">
									<div className="w-8 h-8 rounded-lg bg-white/5 flex items-center justify-center border border-white/10">
										<BrainCircuit className="w-5 h-5 text-white" />
									</div>
									<span className="text-sm font-normal text-white">Agent Actor: uuid-402</span>
								</div>
								<div className="px-2 py-0.5 rounded-full bg-[#FF4500]/10 text-[#FF4500] text-[10px] font-medium border border-[#FF4500]/20 uppercase tracking-wider">Active</div>
							</div>

							<div className="grid grid-cols-2 gap-4 flex-1">
								<div className="rounded-lg border border-white/5 bg-white/5 p-4 flex flex-col gap-2">
									<div className="flex items-center gap-2 text-zinc-400 font-mono text-[10px] uppercase tracking-wider">
										<Database className="w-3 h-3" /> State
									</div>
									<div className="flex-1 bg-black/40 rounded p-2 font-mono text-[9px] text-zinc-500 overflow-hidden text-left leading-relaxed">
										{`history: [...] \ncontext_window: 12k \nactive_goal: "refactor"`}
									</div>
								</div>
								<div className="rounded-lg border border-white/5 bg-white/5 p-4 flex flex-col gap-2">
									<div className="flex items-center gap-2 text-zinc-400 font-mono text-[10px] uppercase tracking-wider">
										<Microchip className="w-3 h-3" /> Reasoning
									</div>
									<div className="flex-1 flex flex-col items-center justify-center gap-2">
										<div className="w-full h-1 bg-zinc-800 rounded-full overflow-hidden">
											<div className="h-full bg-[#FF4500] w-1/3 animate-[pulse_2s_infinite]" />
										</div>
										<span className="text-[9px] text-zinc-500">Planning step...</span>
									</div>
								</div>
								<div className="rounded-lg border border-white/5 bg-white/5 p-3 col-span-2 flex items-center gap-4">
									<div className="text-zinc-400 font-mono text-[10px] uppercase tracking-wider flex items-center gap-2 whitespace-nowrap">
										<Terminal className="w-3 h-3" /> Active Tools:
									</div>
									<div className="flex gap-2">
										<div className="px-2 py-1 rounded bg-zinc-900 border border-zinc-700 text-[9px] text-zinc-400">GIT-WORKSPACE</div>
										<div className="px-2 py-1 rounded bg-zinc-900 border border-zinc-700 text-[9px] text-zinc-400">SHELL-EXEC</div>
									</div>
								</div>
							</div>
						</div>

						{/* Connection Lines */}
						<div className="absolute left-8 top-1/2 right-1/2 mr-[225px] h-[1px] bg-zinc-700 -translate-y-1/2" />
						<div className="absolute left-10 top-[calc(50%-12px)] text-[10px] font-mono text-zinc-600">CLIENT REQUEST</div>

						<div className="absolute right-8 top-1/2 left-1/2 ml-[225px] h-[1px] bg-zinc-700 -translate-y-1/2" />
						<div className="absolute right-10 top-[calc(50%-12px)] text-[10px] font-mono text-zinc-600">TOOL RESPONSE</div>
					</motion.div>
				</div>
			</div>
		</section>
	);
};

const AgentFeatures = () => {
	const features = [
		{ title: "Long-Running Tasks", description: "Agents can 'hibernate' while waiting for external API callbacks or human approval, consuming zero resources.", icon: Clock },
		{ title: "Realtime", description: "Stream responses and updates in real-time via WebSockets. Agents can broadcast events, progress updates, and intermediate results as they work.", icon: Wifi },
		{ title: "Durable Execution", description: "Actors persist their thought process. If the server restarts, the agent resumes exactly where it left off, mid-thought.", icon: RefreshCw },
		{ title: "Orchestration", description: "Communicate between agents using low-latency RPC. Agents can 'hand off' state to each other effortlessly.", icon: ArrowLeftRight },
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
							Why Actors for Agents?
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-base leading-relaxed text-zinc-500"
						>
							Rivet Actors are the fundamental building blocks for agents that need to reason about state over time.
						</motion.p>
					</div>

					<div className="grid grid-cols-1 gap-8 md:grid-cols-2">
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
						className="mb-3 text-2xl font-normal tracking-tight text-white md:text-4xl"
					>
						Autonomous Coding Agent
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="mb-8 text-base leading-relaxed text-zinc-500"
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
									<FileCode className="h-4 w-4 text-white" />
								</div>
								<div>
									<div className="text-sm font-normal text-white">Task: Fix CVE-2024-81</div>
									<div className="text-xs text-zinc-500">Sub-processes: 2</div>
								</div>
							</div>
							<div className="rounded-full border border-white/10 bg-white/5 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider text-zinc-400">Monitoring</div>
						</div>
						<div className="space-y-4 text-sm font-mono">
							<div className="rounded border border-white/5 bg-zinc-950 p-3 text-zinc-400">
								<div className="flex justify-between mb-2">
									<span className="text-xs text-zinc-500">SHELL</span>
									<span className="text-[10px] text-[#FF4500]">REAL-TIME</span>
								</div>
								<div className="text-[10px] text-zinc-500">
									$ npm test --grep security-patch <br/>
									<span className="text-zinc-400">✓ Patch verified (12 tests passed)</span>
								</div>
							</div>
							<div className="flex justify-between rounded border border-white/5 bg-zinc-950 p-3 text-zinc-400">
								<span>Linter</span>
								<span className="text-[10px] text-zinc-500">Waiting...</span>
							</div>
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
						Integrates with
					</motion.h2>
				</div>
				<div className="flex flex-wrap gap-2">
					{["LangChain", "LlamaIndex", "Vercel AI SDK", "OpenAI", "Anthropic", "Hugging Face"].map((tech, i) => (
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

export default function AgentsPage() {
	return (
		<div className="min-h-screen bg-black font-sans text-zinc-300 selection:bg-[#FF4500]/30 selection:text-orange-200">
			<main>
				<Hero />
				<AgentArchitecture />
				<AgentFeatures />
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
							Build smarter agents.
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="mx-auto mb-8 max-w-lg text-base leading-relaxed text-zinc-500"
						>
							Give your AI the stateful foundation it needs to actually solve complex problems.
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
