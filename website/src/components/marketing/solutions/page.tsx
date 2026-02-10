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
const Badge = ({ text }: { text: string }) => (
	<div className="inline-flex items-center gap-2 rounded-full border border-white/10 px-3 py-1 text-xs text-zinc-400 mb-6">
		<span className="h-1.5 w-1.5 rounded-full bg-[#FF4500]" />
		{text}
	</div>
);

const CodeBlock = ({ code, fileName = "agent.ts" }: { code: string; fileName?: string }) => {
	const highlightLine = (line: string) => {
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

			if (["import", "from", "export", "const", "return", "async", "await", "function", "let", "var", "if", "else", "while", "true", "false", "null"].includes(trimmed)) {
				tokens.push(<span key={j} className="text-purple-400">{part}</span>);
			}
			else if (["actor", "spawn", "rpc", "ai"].includes(trimmed)) {
				tokens.push(<span key={j} className="text-blue-400">{part}</span>);
			}
			else if (["state", "actions", "broadcast", "c", "run", "queue", "next", "push", "history", "messages"].includes(trimmed)) {
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
	};

	return (
		<div className="relative rounded-lg overflow-hidden border border-white/10 bg-black">
			<div className="flex items-center justify-between px-4 py-2 border-b border-white/10 bg-white/5">
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
								<span className="table-cell">{highlightLine(line)}</span>
							</div>
						))}
					</code>
				</pre>
			</div>
		</div>
	);
};

// --- Simple Feature Card ---
const FeatureItem = ({ title, description, icon: Icon }: { title: string; description: string; icon: typeof Database }) => (
	<div className="border-t border-white/10 pt-6">
		<div className="mb-3 text-zinc-500">
			<Icon className="h-4 w-4" />
		</div>
		<h3 className="mb-1 text-sm font-normal text-white">{title}</h3>
		<p className="text-sm leading-relaxed text-zinc-500">{description}</p>
	</div>
);

// --- Page Sections ---
const Hero = () => (
	<section className="relative overflow-hidden pb-20 pt-32 md:pb-32 md:pt-48">
		<div className="mx-auto max-w-7xl px-6">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<Badge text="Rivet for Agents" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="mb-6 text-4xl font-normal leading-[1.1] tracking-tight text-white md:text-6xl"
					>
						Build AI Agents
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="mb-8 max-w-lg text-base leading-relaxed text-zinc-500"
					>
						LLMs are stateless. Agents shouldn't be. Rivet Actors provide the persistent memory, tool execution environment, and long-running context your agents need to thrive.
					</motion.p>
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="flex flex-col sm:flex-row items-center gap-3"
					>
						<a href="/docs" className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200">
							Start Building
							<ArrowRight className="h-4 w-4" />
						</a>
						<a href="/docs/agents" className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white">
							View Documentation
						</a>
					</motion.div>
				</div>
				<div className="flex-1 w-full max-w-xl">
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
		<section className="border-t border-white/10 py-48">
			<div className="mx-auto max-w-7xl px-6">
				<div className="mb-12">
					<h2 className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl">
						Why Actors for Agents?
					</h2>
					<p className="max-w-2xl text-base leading-relaxed text-zinc-500">
						Traditional architectures force you to fetch conversation history from a database for every single token generated. Rivet Actors keep the context hot in RAM.
					</p>
				</div>

				<div className="grid lg:grid-cols-2 gap-12 items-center">
					{/* Diagram */}
					<div className="relative h-80 rounded-lg border border-white/10 bg-black flex items-center justify-center overflow-hidden">
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
					<div className="space-y-8">
						<motion.div
							initial={{ opacity: 0, x: 20 }}
							whileInView={{ opacity: 1, x: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5 }}
							className="border-t border-white/10 pt-6"
						>
							<div className="mb-3 text-zinc-500">
								<Database className="w-4 h-4" />
							</div>
							<h3 className="mb-1 text-sm font-normal text-white">Zero-Latency Context</h3>
							<p className="text-zinc-500 text-sm leading-relaxed">
								Conversation history and embedding vectors stay in the Actor's heap. No database queries required to "rehydrate" the agent state for each message.
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
								<Clock className="w-4 h-4" />
							</div>
							<h3 className="mb-1 text-sm font-normal text-white">Long-Running "Thought" Loops</h3>
							<p className="text-zinc-500 text-sm leading-relaxed">
								Agents often need to chain multiple tool calls (CoT). Actors can run for minutes or hours, maintaining their state throughout the entire reasoning chain without timeouts.
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
								<Users className="w-4 h-4" />
							</div>
							<h3 className="mb-1 text-sm font-normal text-white">Multi-User Collaboration</h3>
							<p className="text-zinc-500 text-sm leading-relaxed">
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
		},
		{
			title: "Tool Execution Sandbox",
			description: "Actors provide a secure isolation boundary. Run Python scripts or API calls without blocking your main API fleet.",
			icon: Terminal,
		},
		{
			title: "Human-in-the-Loop",
			description: "Pause execution, wait for a human approval signal via RPC, and then resume the agent's context exactly where it left off.",
			icon: Users,
		},
		{
			title: "Scheduled Wake-ups",
			description: "Set an alarm for your agent. It can sleep to disk and wake up in 2 days to follow up with a user automatically.",
			icon: Clock,
		},
		{
			title: "Knowledge Graph State",
			description: "Keep a complex graph of entities in memory. Update relationships dynamically as the conversation progresses.",
			icon: Network,
		},
		{
			title: "Vendor Neutral",
			description: "Swap between OpenAI, Anthropic, or local Llama models instantly. The Actor pattern abstracts the underlying intelligence.",
			icon: Globe,
		},
	];

	return (
		<section className="border-t border-white/10 py-48">
			<div className="mx-auto max-w-7xl px-6">
				<div className="flex flex-col gap-12">
					<div className="max-w-xl">
						<h2 className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl">Built for the Agentic Future</h2>
						<p className="text-base leading-relaxed text-zinc-500">The infrastructure primitives you need to move beyond simple chatbots.</p>
					</div>

					<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
						{capabilities.map((cap, idx) => (
							<FeatureItem key={idx} {...cap} />
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
					<Badge text="Case Study" />
					<h2 className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl">
						Customer Support Swarms
					</h2>
					<p className="mb-8 text-base leading-relaxed text-zinc-500">
						Deploy a dedicated Actor for every single active support ticket.
					</p>
					<ul className="space-y-4">
						{["Isolation: One crashed agent doesn't affect others", "Context: Full ticket history in memory (up to 128k tokens)", "Handoff: Transfer actor state to a human instantly"].map((item, i) => (
							<li key={i} className="flex items-center gap-3 text-sm text-zinc-300">
								<div className="flex h-5 w-5 items-center justify-center rounded-full border border-white/10 bg-white/5">
									<Check className="h-3 w-3 text-[#FF4500]" />
								</div>
								{item}
							</li>
						))}
					</ul>
				</div>
				<div className="relative rounded-lg border border-white/10 bg-black p-6">
					<div className="flex items-center justify-between mb-6 border-b border-white/5 pb-4">
						<div className="flex items-center gap-3">
							<div className="flex h-8 w-8 items-center justify-center rounded-lg border border-white/10 bg-white/5">
								<Bot className="h-4 w-4 text-white" />
							</div>
							<div>
								<div className="text-sm font-medium text-white">Ticket #9402</div>
								<div className="text-xs text-zinc-500">Agent: Active</div>
							</div>
						</div>
						<div className="rounded border border-[#FF4500]/20 bg-[#FF4500]/10 px-2 py-1 text-xs text-[#FF4500]">Online</div>
					</div>
					<div className="space-y-3 font-mono text-sm">
						<div className="rounded border border-white/5 bg-zinc-900 p-3 text-zinc-400">Analysing user request... accessing database...</div>
						<div className="rounded border border-white/10 bg-white/5 p-3 text-zinc-300">Found 3 matching orders. Asking clarification...</div>
						<div className="flex gap-2">
							<div className="h-2 w-2 animate-bounce rounded-full bg-[#FF4500]" />
							<div className="h-2 w-2 animate-bounce rounded-full bg-[#FF4500] delay-75" />
							<div className="h-2 w-2 animate-bounce rounded-full bg-[#FF4500] delay-150" />
						</div>
					</div>
				</div>
			</div>
		</div>
	</section>
);

const OrchestrationSection = () => {
	return (
		<section className="border-t border-white/10 py-48">
			<div className="mx-auto max-w-7xl px-6">
				<div className="flex flex-col lg:flex-row gap-16 items-center">
					<div className="flex-1 max-w-xl order-2 lg:order-1">
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
					<div className="flex-1 order-1 lg:order-2">
						<Badge text="Multi-Agent Systems" />
						<h2 className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl">
							Orchestrate Agent Swarms
						</h2>
						<p className="mb-8 text-base leading-relaxed text-zinc-500">
							Don't stop at one. Build hierarchical trees of Actors where managers delegate tasks to specialized workers. Rivet handles the supervision, networking, and message passing between them automatically.
						</p>
						<div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
							{[
								{ title: "Parallel Processing", desc: "Run 100 research agents at once." },
								{ title: "Supervision Trees", desc: "Restart workers if they hallucinate or crash." },
								{ title: "Shared State", desc: "Pass context references instantly between actors." },
								{ title: "Event Bus", desc: "Pub/Sub messaging between swarm members." },
							].map((item, i) => (
								<div key={i} className="border-t border-white/10 pt-4">
									<h4 className="text-sm font-medium text-white mb-1">{item.title}</h4>
									<p className="text-xs text-zinc-500">{item.desc}</p>
								</div>
							))}
						</div>
					</div>
				</div>
			</div>
		</section>
	);
};

const Ecosystem = () => (
	<section className="border-t border-white/10 py-48">
		<div className="mx-auto max-w-7xl px-6 text-center">
			<h2 className="mb-8 text-2xl font-normal tracking-tight text-white md:text-4xl">
				Works with your stack
			</h2>
			<div className="flex flex-wrap justify-center gap-2">
				{["LangChain", "LlamaIndex", "Vercel AI SDK", "OpenAI", "Anthropic", "HuggingFace"].map((tech) => (
					<div
						key={tech}
						className="rounded-md border border-white/5 px-2 py-1 text-xs text-zinc-400 transition-colors hover:border-white/20 hover:text-white"
					>
						{tech}
					</div>
				))}
			</div>
		</div>
	</section>
);

export default function AgentsPage() {
	return (
		<div className="min-h-screen bg-black font-sans text-zinc-300 selection:bg-[#FF4500]/30 selection:text-orange-200">
			<main>
				<Hero />
				<MemoryArchitecture />
				<AgentCapabilities />
				<UseCases />
				<OrchestrationSection />
				<Ecosystem />

				{/* CTA Section */}
				<section className="border-t border-white/10 py-48">
					<div className="mx-auto max-w-3xl px-6 text-center">
						<h2 className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl">
							Stop building generic bots.
						</h2>
						<p className="mb-8 text-base leading-relaxed text-zinc-500">
							Start building stateful, durable, intelligent agents that actually work.
						</p>
						<div className="flex flex-col items-center justify-center gap-3 sm:flex-row">
							<a href="/docs" className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200">
								Start Building
								<ArrowRight className="h-4 w-4" />
							</a>
							<a href="/docs/actors" className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white">
								Read the Docs
							</a>
						</div>
					</div>
				</section>
			</main>
		</div>
	);
}

