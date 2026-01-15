"use client";

import { useState } from "react";
import {
	Terminal,
	Zap,
	Globe,
	ArrowRight,
	Database,
	Check,
	Cpu,
	RefreshCw,
	Clock,
	Shield,
	Cloud,
	Activity,
	Wifi,
	Lock,
	Network,
	HardDrive,
	Users,
	Sparkles,
	Plug,
	Link as LinkIcon,
} from "lucide-react";
import { motion } from "framer-motion";

// --- Shared Design Components ---
const Badge = ({ text, color = "indigo" }) => {
	const colorClasses = {
		orange: "text-orange-400 border-orange-500/20 bg-orange-500/10",
		blue: "text-blue-400 border-blue-500/20 bg-blue-500/10",
		red: "text-red-400 border-red-500/20 bg-red-500/10",
		zinc: "text-zinc-400 border-zinc-500/20 bg-zinc-500/10",
		indigo: "text-indigo-400 border-indigo-500/20 bg-indigo-500/10",
	};

	return (
		<div
			className={`inline-flex items-center gap-2 px-3 py-1 rounded-full border text-xs font-medium mb-8 transition-colors cursor-default ${colorClasses[color]}`}
		>
			<span className={`w-1.5 h-1.5 rounded-full ${color === "orange" ? "bg-orange-400" : color === "blue" ? "bg-blue-400" : color === "red" ? "bg-red-400" : color === "zinc" ? "bg-zinc-400" : "bg-indigo-400"} animate-pulse`} />
			{text}
		</div>
	);
};

const CodeBlock = ({ code, fileName = "mcp_server.ts" }) => {
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

											if (["import", "from", "export", "const", "return", "async", "await", "function", "let", "var"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-purple-400">{part}</span>);
											} else if (["actor", "McpServer", "tool", "connect", "z"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-400">{part}</span>);
											} else if (["state", "actions", "preferences", "history", "name", "version", "key", "val", "content", "type", "text", "server", "transport"].includes(trimmed)) {
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

const SolutionCard = ({ title, description, icon: Icon, color = "indigo" }) => {
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
			case "indigo":
				return {
					bg: "bg-indigo-500/10",
					text: "text-indigo-400",
					hoverBg: "group-hover:bg-indigo-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(99,102,241,0.5)]",
					border: "border-indigo-500",
					glow: "rgba(99,102,241,0.15)",
				};
			default:
				return {
					bg: "bg-indigo-500/10",
					text: "text-indigo-400",
					hoverBg: "group-hover:bg-indigo-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(99,102,241,0.5)]",
					border: "border-indigo-500",
					glow: "rgba(99,102,241,0.15)",
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
		<div className="absolute top-0 left-1/2 -translate-x-1/2 w-[1000px] h-[500px] bg-indigo-500/[0.03] blur-[100px] rounded-full pointer-events-none" />

		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<Badge text="Model Context Protocol" color="indigo" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="text-5xl md:text-7xl font-medium text-white tracking-tight leading-[1.1] mb-6"
					>
						The Stateful <br />
						<span className="text-indigo-400">MCP Server.</span>
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg md:text-xl text-zinc-400 leading-relaxed mb-8 max-w-lg"
					>
						Don't just connect LLMs to tools. Connect them to <em>state</em>. Rivet Actors let you deploy persistent, user-aware MCP servers that remember everything.
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
						<div className="absolute -inset-1 bg-gradient-to-r from-indigo-500/20 to-blue-500/20 rounded-xl blur opacity-40" />
						<CodeBlock
							fileName="mcp_server.ts"
							code={`import { actor } from "rivetkit";
import { McpServer } from "@modelcontextprotocol/sdk";

// One dedicated MCP server per user
export const userContext = actor({
  state: { preferences: {}, history: [] },
  actions: {
    connect: async (c) => {
      const server = new McpServer({
        name: "PersonalContext",
        version: "1.0.0"
      });
      
      // Tool that reads/writes persistent state
      server.tool("remember", { key: z.string(), val: z.string() }, 
        async ({ key, val }) => {
          c.state.preferences[key] = val;
          return { content: [{ type: "text", text: "Saved." }] };
        }
      );
      
      return server.connect(c.transport);
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

const ProtocolArchitecture = () => {
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
						Stateful vs. Stateless MCP
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-zinc-400 max-w-2xl mx-auto text-lg leading-relaxed"
					>
						Standard MCP servers are often stateless processes. Rivet Actors give each connection its own long-lived memory, enabling personalized and continuous AI interactions.
					</motion.p>
				</div>

				<div className="relative h-[400px] w-full rounded-2xl border border-white/10 bg-zinc-900/20 flex items-center justify-center overflow-hidden p-8">
					<div className="absolute inset-0 bg-[linear-gradient(rgba(255,255,255,0.03)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.03)_1px,transparent_1px)] bg-[size:40px_40px]" />

					<div className="relative z-10 w-full max-w-5xl grid grid-cols-3 gap-8 items-center">
						{/* Left: Client (Claude/Cursor) */}
						<div className="flex flex-col items-center gap-4">
							<div className="w-20 h-20 rounded-2xl bg-zinc-950 border border-zinc-700 flex items-center justify-center shadow-lg relative">
								<div className="absolute -top-3 px-2 py-0.5 bg-zinc-800 text-[10px] rounded-full text-zinc-400 border border-zinc-700">Client</div>
								<Sparkles className="w-8 h-8 text-indigo-400" />
							</div>
							<span className="text-zinc-500 font-mono text-xs uppercase tracking-widest">Claude / Cursor</span>
						</div>

						{/* Middle: The Protocol Pipe */}
						<div className="relative h-12 w-full bg-zinc-900/50 border-y border-zinc-800 flex items-center justify-center">
							<div className="absolute inset-0 bg-indigo-500/5 animate-pulse" />
							<div className="flex gap-16 text-[10px] font-mono text-zinc-500 uppercase tracking-widest z-10">
								<span>JSON-RPC</span>
								<span>SSE</span>
							</div>
						</div>

						{/* Right: Rivet Actor */}
						<div className="flex flex-col items-center gap-4 relative">
							<div className="w-24 h-24 rounded-2xl bg-zinc-950 border border-indigo-500/50 flex flex-col items-center justify-center shadow-[0_0_40px_rgba(99,102,241,0.2)] relative overflow-hidden">
								<div className="absolute inset-0 bg-[radial-gradient(circle_at_center,rgba(99,102,241,0.1),transparent)]" />
								<Database className="w-8 h-8 text-indigo-500 mb-2 relative z-10" />
								<div className="flex gap-1">
									<div className="w-1 h-1 bg-indigo-400 rounded-full animate-bounce" />
									<div className="w-1 h-1 bg-indigo-400 rounded-full animate-bounce delay-100" />
									<div className="w-1 h-1 bg-indigo-400 rounded-full animate-bounce delay-200" />
								</div>
							</div>
							<span className="text-indigo-400 font-mono text-xs uppercase tracking-widest">Stateful Actor</span>
						</div>
					</div>
				</div>
			</div>
		</section>
	);
};

const MCPFeatures = () => {
	const features = [
		{
			title: "User-Specific Servers",
			description: "Spawn a unique MCP server for every user. Give them their own memory, preferences, and authentication context.",
			icon: Users,
			color: "indigo",
		},
		{
			title: "Persistent Memory",
			description: "Tools can write to `c.state`. Data survives server restarts and is instantly available next time the user connects.",
			icon: HardDrive,
			color: "blue",
		},
		{
			title: "Long-Running Tools",
			description: "Execute tools that take minutes or hours (e.g. scraping, compiling). The Actor stays alive while the LLM waits.",
			icon: Clock,
			color: "orange",
		},
		{
			title: "SSE & Stdio Support",
			description: "Connect via HTTP Server-Sent Events for web agents, or Stdio for local desktop apps like Claude.",
			icon: Plug,
			color: "zinc",
		},
		{
			title: "Secure Headers",
			description: "Pass authentication tokens from the client directly to the Actor. Secure your tools with per-user permissions.",
			icon: Lock,
			color: "red",
		},
		{
			title: "Instant Wake",
			description: "Your MCP server hibernates when unused (0 cost) and wakes up in milliseconds when the LLM calls a tool.",
			icon: Zap,
			color: "indigo",
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
					<h2 className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight">Protocol Superpowers</h2>
					<p className="text-zinc-400 text-lg leading-relaxed">Enhance the Model Context Protocol with durable compute.</p>
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
					<Badge text="Case Study" color="indigo" />
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						Persistent Cloud IDE
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg text-zinc-400 mb-8 leading-relaxed"
					>
						Deploy a dedicated MCP server for each developer that maintains project context, terminal history, and language server state across sessions.
					</motion.p>
					<ul className="space-y-4">
						{[
							"Hot-Swappable: Update tools on the fly without restarting context",
							"Session Recall: 'Remember that bug from Tuesday?' actually works",
							"Secure Tunnel: Authenticated connection between local editor and cloud actor",
						].map((item, i) => (
							<motion.li
								key={i}
								initial={{ opacity: 0, x: -20 }}
								whileInView={{ opacity: 1, x: 0 }}
								viewport={{ once: true }}
								transition={{ duration: 0.5, delay: 0.2 + i * 0.1 }}
								className="flex items-center gap-3 text-zinc-300"
							>
								<div className="w-5 h-5 rounded-full bg-indigo-500/20 flex items-center justify-center">
									<Check className="w-3 h-3 text-indigo-400" />
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
					<div className="absolute inset-0 bg-gradient-to-r from-indigo-500/20 to-transparent rounded-2xl blur-2xl" />
					<div className="relative rounded-2xl border border-white/10 bg-zinc-900 p-6 shadow-2xl">
						<div className="flex items-center justify-between mb-6 border-b border-white/5 pb-4">
							<div className="flex items-center gap-3">
								<div className="w-8 h-8 rounded bg-indigo-500/20 flex items-center justify-center">
									<Terminal className="w-5 h-5 text-indigo-400" />
								</div>
								<div>
									<div className="text-sm font-medium text-white">DevSession #8492</div>
									<div className="text-xs text-zinc-500">Status: Listening</div>
								</div>
							</div>
							<div className="px-2 py-1 rounded bg-green-500/10 text-green-400 text-xs border border-green-500/20">Connected</div>
						</div>
						<div className="space-y-4 text-sm font-mono">
							<div className="p-3 rounded bg-zinc-950 border border-white/5 text-zinc-400">&gt; User: Refactor auth.ts based on yesterday's logs.</div>
							<div className="p-3 rounded bg-indigo-900/20 border border-indigo-500/30 text-indigo-200">&lt; MCP: Retrieving logs from Actor state... Context found. Refactoring.</div>
							<div className="flex gap-2">
								<div className="h-1.5 w-full bg-zinc-800 rounded-full overflow-hidden">
									<div className="h-full bg-indigo-500 w-3/4 animate-pulse" />
								</div>
							</div>
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
			title: "Personalized Memory",
			desc: "An MCP server that learns user preferences over time and stores them in the Actor state.",
		},
		{
			title: "Dev Environment",
			desc: "A persistent cloud coding environment that an LLM can manipulate via MCP tools.",
		},
		{
			title: "Async Research",
			desc: "Trigger a research task via MCP, let the Actor run for an hour, and notify the LLM when done.",
		},
		{
			title: "Team Knowledge",
			desc: "A shared MCP server that multiple team members (and their LLMs) connect to simultaneously.",
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
					Unlock New Capabilities
				</motion.h2>
				<div className="grid md:grid-cols-2 lg:grid-cols-4 gap-6">
					{cases.map((c, i) => (
						<motion.div
							key={i}
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: i * 0.05 }}
							className="p-6 rounded-xl border border-white/10 bg-zinc-900/30 hover:bg-indigo-900/10 hover:border-indigo-500/30 transition-colors group"
						>
							<div className="mb-4">
								<Network className="w-6 h-6 text-indigo-500 group-hover:scale-110 transition-transform" />
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
				{["Claude Desktop", "Cursor", "Zed", "Vercel AI SDK", "LangChain", "Sourcegraph Cody"].map((tech, i) => (
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

export default function MCPPage() {
	return (
		<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-indigo-500/30 selection:text-indigo-200">
			<main>
				<Hero />
				<ProtocolArchitecture />
				<MCPFeatures />
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
							Give your AI a brain.
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-lg text-zinc-400 mb-10 leading-relaxed"
						>
							Start building stateful MCP servers that actually remember.
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

