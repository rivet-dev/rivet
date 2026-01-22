"use client";

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
	HardDrive,
	Container,
	Coins,
	Cpu as Chip,
	Wand2,
} from "lucide-react";
import { motion } from "framer-motion";

// --- Shared Design Components ---
const Badge = ({ text, color = "pink" }) => {
	const colorClasses = {
		orange: "text-orange-400 border-orange-500/20 bg-orange-500/10",
		blue: "text-blue-400 border-blue-500/20 bg-blue-500/10",
		purple: "text-purple-400 border-purple-500/20 bg-purple-500/10",
		pink: "text-pink-400 border-pink-500/20 bg-pink-500/10",
	};

	return (
		<div
			className={`inline-flex items-center gap-2 px-3 py-1 rounded-full border text-xs font-medium mb-8 transition-colors cursor-default ${colorClasses[color]}`}
		>
			<span className={`w-1.5 h-1.5 rounded-full ${color === "orange" ? "bg-orange-400" : color === "blue" ? "bg-blue-400" : color === "purple" ? "bg-purple-400" : "bg-pink-400"} animate-pulse`} />
			{text}
		</div>
	);
};

const CodeBlock = ({ code, fileName = "platform.ts" }) => {
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
											else if (["actor", "broadcast", "deployGeneratedApp", "getApp"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-400">{part}</span>);
											}
											// Object Keys / Properties / Methods
											else if (["state", "actions", "runningApps", "code", "appId", "status"].includes(trimmed)) {
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

// --- Refined Feature Card matching landing page style with color highlights ---
const SolutionCard = ({ title, description, icon: Icon, color = "pink" }) => {
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
			case "pink":
				return {
					bg: "bg-pink-500/10",
					text: "text-pink-400",
					hoverBg: "group-hover:bg-pink-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(236,72,153,0.5)]",
					border: "border-pink-500",
					glow: "rgba(236,72,153,0.15)",
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
					bg: "bg-pink-500/10",
					text: "text-pink-400",
					hoverBg: "group-hover:bg-pink-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(236,72,153,0.5)]",
					border: "border-pink-500",
					glow: "rgba(236,72,153,0.15)",
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
		<div className="absolute top-0 left-1/2 -translate-x-1/2 w-[1000px] h-[500px] bg-pink-500/[0.03] blur-[100px] rounded-full pointer-events-none" />

		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<Badge text="For AI Platforms" color="pink" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="text-5xl md:text-7xl font-medium text-white tracking-tight leading-[1.1] mb-6"
					>
						The Backend for <br />
						<span className="text-pink-400">App Generators.</span>
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg md:text-xl text-zinc-400 leading-relaxed mb-8 max-w-lg"
					>
						Don't burn tokens managing database schemas in your context window. Give every generated app its own isolated, stateful Actor instantly.
					</motion.p>
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="flex flex-col sm:flex-row items-center gap-4"
					>
						<a href="https://dashboard.rivet.dev/" className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black shadow-sm hover:bg-zinc-200 transition-colors gap-2">
							Start Building
							<ArrowRight className="w-4 h-4" />
						</a>
					</motion.div>
				</div>
				<div className="flex-1 w-full max-w-xl">
					<div className="relative">
						<div className="absolute -inset-1 bg-gradient-to-r from-pink-500/20 to-purple-500/20 rounded-xl blur opacity-40" />
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
		</div>
	</section>
);

const TokenEfficiencyVisualizer = () => {
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
						Stop Paying for State
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-zinc-400 max-w-2xl mx-auto text-lg leading-relaxed"
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
						className="p-8 rounded-2xl border border-white/10 bg-zinc-900/30 flex flex-col items-center"
					>
						<h3 className="text-zinc-500 font-mono text-sm uppercase tracking-widest mb-6">Traditional LLM Backend</h3>

						{/* Stack Visualization */}
						<div className="w-48 flex flex-col gap-1 relative">
							{/* Context Window (Bloated) */}
							<div className="h-64 w-full bg-zinc-800 rounded-lg border border-zinc-700 flex flex-col p-2 gap-1 overflow-hidden relative">
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
						className="p-8 rounded-2xl border border-pink-500/30 bg-pink-900/5 flex flex-col items-center relative overflow-hidden"
					>
						<div className="absolute inset-0 bg-gradient-to-b from-pink-500/5 to-transparent pointer-events-none" />
						<h3 className="text-pink-400 font-mono text-sm uppercase tracking-widest mb-6">Rivet Actor Backend</h3>

						{/* Stack Visualization */}
						<div className="w-48 flex flex-col gap-1 relative">
							{/* Context Window (Lean) */}
							<div className="h-24 w-full bg-zinc-800 rounded-lg border border-pink-500/50 flex flex-col p-2 gap-1 shadow-[0_0_30px_rgba(236,72,153,0.15)]">
								<div className="flex-1 bg-green-500/20 border border-green-500/30 rounded flex items-center justify-center text-[10px] text-green-300 font-medium">Pure Logic</div>
							</div>

							{/* The Actor State (Offloaded) */}
							<div className="h-32 w-full mt-4 rounded-xl border border-dashed border-zinc-600 flex flex-col items-center justify-center bg-black/50 p-2 gap-2">
								<div className="flex items-center gap-2 text-zinc-400">
									<HardDrive className="w-4 h-4" />
									<span className="text-[10px]">Actor Memory</span>
								</div>
								<div className="w-full h-1 bg-zinc-700 rounded-full overflow-hidden">
									<div className="h-full bg-pink-500 w-2/3 animate-pulse" />
								</div>
								<span className="text-[10px] text-zinc-500">State persisted automatically</span>
							</div>
							<span className="text-center text-green-400 text-xs font-medium mt-2">⚡ Low Cost / Fast</span>
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
			color: "pink",
		},
		{
			title: "Zero-Config State",
			description: "Your users don't need to setup Postgres. `state.count++` is persisted instantly.",
			icon: Database,
			color: "blue",
		},
		{
			title: "Token Efficient",
			description: "Reduce prompt size by 80%. Don't send the DB schema with every request; just send the logic.",
			icon: Coins,
			color: "purple",
		},
		{
			title: "Instant Deploy",
			description: "Spawn a new backend in <10ms. Perfect for 'Click to Run' AI interfaces.",
			icon: Zap,
			color: "orange",
		},
		{
			title: "Hot Swappable",
			description: "Update the actor's behavior in real-time as the AI generates new code versions.",
			icon: RefreshCw,
			color: "zinc",
		},
		{
			title: "Streaming Outputs",
			description: "Pipe stdout/stderr from the actor directly to your user's browser via WebSockets.",
			icon: Activity,
			color: "pink",
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
					<h2 className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight">Infrastructure for Generation</h2>
					<p className="text-zinc-400 text-lg leading-relaxed">Primitives designed for AI code generation platforms.</p>
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
		<section className="py-24 bg-black border-t border-white/5">
			<div className="max-w-7xl mx-auto px-6">
				<motion.h2
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="text-3xl md:text-5xl font-medium text-white mb-12 text-center tracking-tight"
				>
					Powering the Next Gen
				</motion.h2>
				<div className="grid md:grid-cols-2 lg:grid-cols-4 gap-6">
					{cases.map((c, i) => (
						<motion.div
							key={i}
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: i * 0.05 }}
							className="p-6 rounded-xl border border-white/10 bg-zinc-900/30 hover:bg-pink-900/10 hover:border-pink-500/30 transition-colors group"
						>
							<div className="mb-4">
								<Chip className="w-6 h-6 text-pink-500 group-hover:scale-110 transition-transform" />
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
				{["Vercel AI SDK", "LangChain", "OpenAI", "Anthropic", "Replit", "Sandpack"].map((tech, i) => (
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

export default function AppGeneratorsPage() {
	return (
		<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-pink-500/30 selection:text-pink-200">
			<main>
				<Hero />
				<TokenEfficiencyVisualizer />
				<PlatformFeatures />
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
							Build the platform.
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-lg text-zinc-400 mb-10 leading-relaxed"
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
