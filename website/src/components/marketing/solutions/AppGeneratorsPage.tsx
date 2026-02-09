"use client";

import {
	Zap,
	ArrowRight,
	Database,
	RefreshCw,
	Activity,
	HardDrive,
	Container,
	Coins,
	Cpu as Chip,
} from "lucide-react";
import { motion } from "framer-motion";

// --- Shared Design Components ---
const Badge = ({ text }: { text: string }) => (
	<div className="inline-flex items-center gap-2 rounded-full border border-white/10 px-3 py-1 text-xs text-zinc-400 mb-6">
		<span className="h-1.5 w-1.5 rounded-full bg-[#FF4500]" />
		{text}
	</div>
);

const CodeBlock = ({ code, fileName = "platform.ts" }: { code: string; fileName?: string }) => {
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
			else if (["state", "actions", "broadcast", "c", "runningApps", "appId", "code", "status", "getApp", "deployGeneratedApp"].includes(trimmed)) {
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

// --- Feature Item Component matching landing page style ---
const FeatureItem = ({ title, description, icon: Icon }: { title: string; description: string; icon: typeof Database }) => (
	<div className="border-t border-white/10 pt-6">
		<div className="mb-3 text-zinc-500">
			<Icon className="h-4 w-4" />
		</div>
		<h3 className="text-sm font-normal text-white mb-1">{title}</h3>
		<p className="text-sm leading-relaxed text-zinc-500">{description}</p>
	</div>
);

// --- Page Sections ---
const Hero = () => (
	<section className="relative pt-32 pb-20 md:pt-48 md:pb-32 overflow-hidden">
		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<Badge text="For AI Platforms" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="text-4xl md:text-6xl font-normal text-white tracking-tight leading-[1.1] mb-6"
					>
						The Backend for App Generators
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-base text-zinc-500 leading-relaxed mb-8 max-w-lg"
					>
						Don't burn tokens managing database schemas in your context window. Give every generated app its own isolated, stateful Actor instantly.
					</motion.p>
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="flex flex-col sm:flex-row items-center gap-4"
					>
						<a href="/docs" className="inline-flex items-center justify-center whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200 gap-2">
							Start Building
							<ArrowRight className="w-4 h-4" />
						</a>
					</motion.div>
				</div>
				<div className="flex-1 w-full max-w-xl">
					<CodeBlock
						fileName="platform_api.ts"
						code={`import { actor } from "rivetkit";

// Spawn a new backend for a user's generated app
// cleanly isolated with zero infrastructure overhead
export const appManager = actor({
  state: { runningApps: {} },
  actions: {
    deployGeneratedApp: async (c, { appId, code }) => {
      // 1. Store the generated code
      c.state.runningApps[appId] = { code, status: "ready" };

      // 2. Broadcast to any connected clients
      c.broadcast("app_deployed", { appId });
      return { appId, status: "ready" };
    },

    getApp: (c, appId) => {
      return c.state.runningApps[appId] || null;
    }
  }
});`}
					/>
				</div>
			</div>
		</div>
	</section>
);

const TokenEfficiencyVisualizer = () => {
	return (
		<section className="border-t border-white/10 py-48">
			<div className="max-w-7xl mx-auto px-6">
				<div className="mb-16">
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-2"
					>
						Stop Paying for State
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-base leading-relaxed text-zinc-500 max-w-2xl"
					>
						Traditional AI coding tools waste thousands of tokens passing database schemas and state history back and forth. Rivet Actors hold state in memory, keeping your context window focused on logic.
					</motion.p>
				</div>

				<div className="grid md:grid-cols-2 gap-8 items-center max-w-5xl mx-auto">
					{/* Traditional Way */}
					<motion.div
						initial={{ opacity: 0, x: -20 }}
						whileInView={{ opacity: 1, x: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="p-8 rounded-lg border border-white/10 bg-black flex flex-col items-center"
					>
						<h3 className="text-zinc-500 font-mono text-sm uppercase tracking-widest mb-6">Traditional LLM Backend</h3>

						{/* Stack Visualization */}
						<div className="w-48 flex flex-col gap-1 relative">
							{/* Context Window (Bloated) */}
							<div className="h-64 w-full bg-zinc-800 rounded-lg border border-white/10 flex flex-col p-2 gap-1 overflow-hidden relative">
								<div className="w-full h-8 bg-red-500/20 border border-red-500/30 rounded flex items-center justify-center text-[10px] text-red-300">DB Schema (2k tokens)</div>
								<div className="w-full h-12 bg-red-500/20 border border-red-500/30 rounded flex items-center justify-center text-[10px] text-red-300">User History (4k tokens)</div>
								<div className="w-full h-8 bg-red-500/20 border border-red-500/30 rounded flex items-center justify-center text-[10px] text-red-300">ORM Definitions (2k)</div>
								<div className="flex-1 bg-green-500/20 border border-green-500/30 rounded flex items-center justify-center text-[10px] text-green-300">Actual Logic</div>

								{/* Overflow fade */}
								<div className="absolute bottom-0 left-0 right-0 h-8 bg-gradient-to-t from-zinc-800 to-transparent" />
							</div>
							<span className="text-center text-red-400 text-xs font-medium mt-2">$$$ High Cost / Slow</span>
						</div>
					</motion.div>

					{/* Rivet Way */}
					<motion.div
						initial={{ opacity: 0, x: 20 }}
						whileInView={{ opacity: 1, x: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="p-8 rounded-lg border border-[#FF4500]/30 bg-black flex flex-col items-center relative overflow-hidden"
					>
						<h3 className="text-[#FF4500] font-mono text-sm uppercase tracking-widest mb-6">Rivet Actor Backend</h3>

						{/* Stack Visualization */}
						<div className="w-48 flex flex-col gap-1 relative">
							{/* Context Window (Lean) */}
							<div className="h-24 w-full bg-zinc-800 rounded-lg border border-[#FF4500]/50 flex flex-col p-2 gap-1">
								<div className="flex-1 bg-green-500/20 border border-green-500/30 rounded flex items-center justify-center text-[10px] text-green-300 font-medium">Pure Logic</div>
							</div>

							{/* The Actor State (Offloaded) */}
							<div className="h-32 w-full mt-4 rounded-xl border border-dashed border-zinc-600 flex flex-col items-center justify-center bg-black/50 p-2 gap-2">
								<div className="flex items-center gap-2 text-zinc-400">
									<HardDrive className="w-4 h-4" />
									<span className="text-[10px]">Actor Memory</span>
								</div>
								<div className="w-full h-1 bg-zinc-700 rounded-full overflow-hidden">
									<div className="h-full bg-[#FF4500] w-2/3 animate-pulse" />
								</div>
								<span className="text-[10px] text-zinc-500">State persisted automatically</span>
							</div>
							<span className="text-center text-green-400 text-xs font-medium mt-2">Low Cost / Fast</span>
						</div>
					</motion.div>
				</div>
			</div>
		</section>
	);
};

const PlatformFeatures = () => {
	const features = [
		{
			title: "Sandbox Isolation",
			description: "Every generated app gets its own Actor. Crashes in one user's app never affect the platform.",
			icon: Container,
		},
		{
			title: "Zero-Config State",
			description: "Your users don't need to setup Postgres. `state.count++` is persisted instantly.",
			icon: Database,
		},
		{
			title: "Token Efficient",
			description: "Reduce prompt size by 80%. Don't send the DB schema with every request; just send the logic.",
			icon: Coins,
		},
		{
			title: "Instant Deploy",
			description: "Spawn a new backend in <10ms. Perfect for 'Click to Run' AI interfaces.",
			icon: Zap,
		},
		{
			title: "Hot Swappable",
			description: "Update the actor's behavior in real-time as the AI generates new code versions.",
			icon: RefreshCw,
		},
		{
			title: "Streaming Outputs",
			description: "Pipe stdout/stderr from the actor directly to your user's browser via WebSockets.",
			icon: Activity,
		},
	];

	return (
		<section className="border-t border-white/10 py-48">
			<div className="max-w-7xl mx-auto px-6">
				<motion.div
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="mb-16"
				>
					<h2 className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-2">Infrastructure for Generation</h2>
					<p className="text-base leading-relaxed text-zinc-500">Primitives designed for AI code generation platforms.</p>
				</motion.div>

				<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
					{features.map((feat, idx) => (
						<motion.div
							key={idx}
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: idx * 0.05 }}
						>
							<FeatureItem {...feat} />
						</motion.div>
					))}
				</div>
			</div>
		</section>
	);
};

const UseCases = () => {
	const cases = [
		{
			title: "No-Code Builders",
			desc: "Let users describe an app and deploy a real, stateful backend instantly.",
		},
		{
			title: "Interactive Tutors",
			desc: "Spawn a coding environment for each student where the AI can verify state.",
		},
		{
			title: "Internal Tool Gens",
			desc: "Generate admin panels on the fly that connect to real APIs and persist settings.",
		},
		{
			title: "Game Engines",
			desc: "Allow users to prompt-generate multiplayer game rules that run on authoritative servers.",
		},
	];

	return (
		<section className="border-t border-white/10 py-48">
			<div className="max-w-7xl mx-auto px-6">
				<motion.h2
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-2"
				>
					Powering the Next Gen
				</motion.h2>
				<div className="grid md:grid-cols-2 lg:grid-cols-4 gap-8">
					{cases.map((c, i) => (
						<motion.div
							key={i}
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: i * 0.05 }}
							className="border-t border-white/10 pt-6"
						>
							<div className="mb-3 text-zinc-500">
								<Chip className="h-4 w-4" />
							</div>
							<h4 className="text-sm font-normal text-white mb-1">{c.title}</h4>
							<p className="text-sm leading-relaxed text-zinc-500">{c.desc}</p>
						</motion.div>
					))}
				</div>
			</div>
		</section>
	);
};

const Ecosystem = () => (
	<section className="border-t border-white/10 py-48">
		<div className="max-w-7xl mx-auto px-6 text-center">
			<motion.h2
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5 }}
				className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-2"
			>
				Works with your stack
			</motion.h2>
			<div className="flex flex-wrap justify-center gap-4">
				{["Vercel AI SDK", "LangChain", "OpenAI", "Anthropic", "Replit", "Sandpack"].map((tech, i) => (
					<motion.div
						key={tech}
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: i * 0.05 }}
						className="rounded-md border border-white/5 px-2 py-1 text-xs text-zinc-400 font-mono hover:text-white hover:border-white/20 transition-colors cursor-default"
					>
						{tech}
					</motion.div>
				))}
			</div>
		</div>
	</section>
);

export default function AppGeneratorsPage() {
	return (
		<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-[#FF4500]/30 selection:text-orange-200">
			<main>
				<Hero />
				<TokenEfficiencyVisualizer />
				<PlatformFeatures />
				<UseCases />
				<Ecosystem />

				{/* CTA Section */}
				<section className="border-t border-white/10 py-48 text-center px-6">
					<div className="max-w-3xl mx-auto">
						<motion.h2
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5 }}
							className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-2"
						>
							Build the platform
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-base text-zinc-500 mb-10 leading-relaxed"
						>
							Give your users the power of stateful backends without the complexity.
						</motion.p>
						<motion.div
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.2 }}
							className="flex flex-col sm:flex-row items-center justify-center gap-4"
						>
							<a href="/docs" className="inline-flex items-center justify-center whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200">
								Start Building Now
							</a>
							<a href="/docs/actors" className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white">
								Read the Docs
							</a>
						</motion.div>
					</div>
				</section>
			</main>
		</div>
	);
}
