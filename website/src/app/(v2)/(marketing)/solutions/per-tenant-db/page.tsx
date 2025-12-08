"use client";

import { useState } from "react";
import {
	Terminal,
	Zap,
	Globe,
	ArrowRight,
	Database,
	Layers,
	Check,
	RefreshCw,
	Shield,
	Network,
	FileJson,
	Key,
	Table2,
	Moon,
	Rocket,
	Coins,
	Gauge,
} from "lucide-react";
import { motion } from "framer-motion";

// --- Shared Design Components ---
const Badge = ({ text, color = "violet" }) => {
	const colorClasses = {
		orange: "text-orange-400 border-orange-500/20 bg-orange-500/10",
		blue: "text-blue-400 border-blue-500/20 bg-blue-500/10",
		pink: "text-pink-400 border-pink-500/20 bg-pink-500/10",
		violet: "text-violet-400 border-violet-500/20 bg-violet-500/10",
	};

	return (
		<div
			className={`inline-flex items-center gap-2 px-3 py-1 rounded-full border text-xs font-medium mb-8 transition-colors cursor-default ${colorClasses[color]}`}
		>
			<span className={`w-1.5 h-1.5 rounded-full ${color === "orange" ? "bg-orange-400" : color === "blue" ? "bg-blue-400" : color === "pink" ? "bg-pink-400" : "bg-violet-400"} animate-pulse`} />
			{text}
		</div>
	);
};

const CodeBlock = ({ code, fileName = "tenant.ts" }) => {
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

											if (["import", "from", "export", "const", "return", "async", "await", "function"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-purple-400">{part}</span>);
											} else if (["actor", "Object", "assign", "push", "updateSettings", "addData"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-400">{part}</span>);
											} else if (["state", "actions", "settings", "data", "newSettings", "item", "count", "length"].includes(trimmed)) {
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

const SolutionCard = ({ title, description, icon: Icon, color = "violet" }) => {
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
			case "violet":
				return {
					bg: "bg-violet-500/10",
					text: "text-violet-400",
					hoverBg: "group-hover:bg-violet-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(139,92,246,0.5)]",
					border: "border-violet-500",
					glow: "rgba(139,92,246,0.15)",
				};
			default:
				return {
					bg: "bg-violet-500/10",
					text: "text-violet-400",
					hoverBg: "group-hover:bg-violet-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(139,92,246,0.5)]",
					border: "border-violet-500",
					glow: "rgba(139,92,246,0.15)",
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
		<div className="absolute top-0 left-1/2 -translate-x-1/2 w-[1000px] h-[500px] bg-violet-500/[0.03] blur-[100px] rounded-full pointer-events-none" />

		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<Badge text="The Ultimate Multi-Tenancy" color="violet" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="text-5xl md:text-7xl font-medium text-white tracking-tight leading-[1.1] mb-6"
					>
						Persistent State for <br />
						<span className="text-violet-400">Every Customer.</span>
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg md:text-xl text-zinc-400 leading-relaxed mb-8 max-w-lg"
					>
						Don't leak data between rows. Give every tenant their own isolated Actor with private in-memory state. Zero latency, instant provisioning, and total data sovereignty.
					</motion.p>

					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="flex flex-col sm:flex-row items-center gap-4"
					>
						<button className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black shadow-sm hover:bg-zinc-200 transition-colors gap-2">
							Start Isolating
							<ArrowRight className="w-4 h-4" />
						</button>
					</motion.div>
				</div>

				<div className="flex-1 w-full max-w-xl">
					<div className="relative">
						<div className="absolute -inset-1 bg-gradient-to-r from-violet-500/20 to-blue-500/20 rounded-xl blur opacity-40" />
						<CodeBlock
							fileName="tenant.ts"
							code={`import { actor } from "rivetkit";

export const tenant = actor({
  // Private state is just a JSON object
  state: { settings: {}, data: [] },

  actions: {
    updateSettings: (c, newSettings) => {
      // Direct in-memory modification
      // Persisted automatically
      Object.assign(c.state.settings, newSettings);
      return c.state.settings;
    },

    addData: (c, item) => {
      c.state.data.push(item);
      return { count: c.state.data.length };
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

const IsolationArchitecture = () => {
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
						The Silo Model
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-zinc-400 max-w-2xl mx-auto text-lg leading-relaxed"
					>
						In a traditional SaaS, one bad query from Tenant A can slow down Tenant B. With Rivet, every tenant lives in their own process with their own resources.
					</motion.p>
				</div>

				<div className="relative h-[450px] w-full rounded-2xl border border-white/10 bg-zinc-900/20 flex items-center justify-center overflow-hidden p-8">
					<div className="absolute inset-0 bg-[linear-gradient(rgba(255,255,255,0.03)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.03)_1px,transparent_1px)] bg-[size:40px_40px]" />

					{/* The Router */}
					<div className="absolute top-12 left-1/2 -translate-x-1/2 z-20 flex flex-col items-center gap-2">
						<div className="w-16 h-16 rounded-xl bg-zinc-950 border border-zinc-700 flex items-center justify-center shadow-lg">
							<Network className="w-8 h-8 text-zinc-400" />
						</div>
						<span className="text-[10px] font-mono text-zinc-500 uppercase tracking-widest">Router</span>
					</div>

					{/* Connection Lines */}
					<svg className="absolute inset-0 w-full h-full pointer-events-none" viewBox="0 0 896 450" preserveAspectRatio="xMidYMid meet">
						<defs>
							<linearGradient id="gradA" x1="0%" y1="0%" x2="0%" y2="100%">
								<stop offset="0%" stopColor="transparent" />
								<stop offset="100%" stopColor="#a78bfa" />
							</linearGradient>
							<linearGradient id="gradB" x1="0%" y1="0%" x2="0%" y2="100%">
								<stop offset="0%" stopColor="transparent" />
								<stop offset="100%" stopColor="#3b82f6" />
							</linearGradient>
							<linearGradient id="gradC" x1="0%" y1="0%" x2="0%" y2="100%">
								<stop offset="0%" stopColor="transparent" />
								<stop offset="100%" stopColor="#f472b6" />
							</linearGradient>
						</defs>

						<path d="M 448 112 L 150 320" stroke="url(#gradA)" strokeWidth="2" fill="none" className="opacity-30" />
						<path d="M 448 112 L 448 320" stroke="url(#gradB)" strokeWidth="2" fill="none" className="opacity-30" />
						<path d="M 448 112 L 746 320" stroke="url(#gradC)" strokeWidth="2" fill="none" className="opacity-30" />

						<circle r="3" fill="#a78bfa">
							<animateMotion dur="2s" repeatCount="indefinite" path="M 448 112 L 150 320" />
						</circle>
						<circle r="3" fill="#3b82f6">
							<animateMotion dur="1.5s" repeatCount="indefinite" path="M 448 112 L 448 320" />
						</circle>
					</svg>

					{/* Tenant Silos */}
					<div className="absolute bottom-12 w-full max-w-4xl grid grid-cols-3 gap-8 px-8 z-10">
						{/* Tenant A */}
						<div className="bg-zinc-950 border border-violet-500/30 p-6 rounded-2xl flex flex-col items-center gap-4 relative group hover:border-violet-500 transition-colors">
							<div className="absolute -top-3 px-3 py-0.5 bg-violet-500/10 border border-violet-500/20 rounded-full text-[10px] text-violet-300 font-mono">Tenant A</div>
							<div className="w-12 h-12 rounded-lg bg-violet-900/20 flex items-center justify-center">
								<Database className="w-6 h-6 text-violet-400" />
							</div>
							<div className="w-full h-1 bg-zinc-800 rounded-full overflow-hidden">
								<div className="h-full w-[40%] bg-violet-500" />
							</div>
							<span className="text-[10px] text-zinc-500">12MB • Active</span>
						</div>

						{/* Tenant B */}
						<div className="bg-zinc-950 border border-blue-500/30 p-6 rounded-2xl flex flex-col items-center gap-4 relative group hover:border-blue-500 transition-colors">
							<div className="absolute -top-3 px-3 py-0.5 bg-blue-500/10 border border-blue-500/20 rounded-full text-[10px] text-blue-300 font-mono">Tenant B</div>
							<div className="w-12 h-12 rounded-lg bg-blue-900/20 flex items-center justify-center">
								<Database className="w-6 h-6 text-blue-400" />
							</div>
							<div className="w-full h-1 bg-zinc-800 rounded-full overflow-hidden">
								<div className="h-full w-[80%] bg-blue-500" />
							</div>
							<span className="text-[10px] text-zinc-500">1.4GB • Active</span>
						</div>

						{/* Tenant C - Sleeping */}
						<div className="bg-zinc-950 border border-pink-500/30 p-6 rounded-2xl flex flex-col items-center gap-4 relative group hover:border-pink-500 transition-colors">
							<div className="absolute -top-3 px-3 py-0.5 bg-pink-500/10 border border-pink-500/20 rounded-full text-[10px] text-pink-300 font-mono">Tenant C</div>
							<div className="w-12 h-12 rounded-lg bg-pink-900/20 flex items-center justify-center opacity-50">
								<Moon className="w-6 h-6 text-pink-400 animate-pulse" />
							</div>
							<div className="w-full h-1 bg-zinc-800 rounded-full overflow-hidden">
								<div className="h-full w-0 bg-pink-500" />
							</div>
							<span className="text-[10px] text-zinc-500">0MB • Sleeping</span>
						</div>
					</div>
				</div>
			</div>
		</section>
	);
};

const StateFeatures = () => {
	const features = [
		{
			title: "Zero-Latency Access",
			description: "The state lives hot in memory on the same node as the compute. No network hop to external cache.",
			icon: Zap,
			color: "violet",
		},
		{
			title: "Instant Provisioning",
			description: "Create a new tenant store in milliseconds. Just spawn an actor; no Terraform required.",
			icon: Rocket,
			color: "blue",
		},
		{
			title: "Schema Isolation",
			description: "Every tenant can have a different state shape. Roll out data migrations gradually, tenant by tenant.",
			icon: FileJson,
			color: "pink",
		},
		{
			title: "Connection Limits",
			description: "Stop worrying about Postgres connection limits. Each actor has exclusive access to its own isolated state.",
			icon: Gauge,
			color: "zinc",
		},
		{
			title: "Data Sovereignty",
			description: "Easily export a single tenant's state as a JSON file. Perfect for GDPR takeouts or backups.",
			icon: Shield,
			color: "violet",
		},
		{
			title: "Cost Efficiency",
			description: "Sleeping tenants cost nothing. You only pay for active CPU/RAM when the state is being accessed.",
			icon: Coins,
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
					className="mb-20"
				>
					<h2 className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight">State Superpowers</h2>
					<p className="text-zinc-400 text-lg leading-relaxed">The benefits of embedded state with the scale of the cloud.</p>
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
					<Badge text="Case Study" color="violet" />
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						B2B CRM Platform
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg text-zinc-400 mb-8 leading-relaxed"
					>
						A CRM serving 10,000 companies. Each company has custom fields, unique workflows, and strict data isolation requirements.
					</motion.p>
					<ul className="space-y-4">
						{["Noisy Neighbor Protection: Large imports by Company A don't slow down Company B", "Custom Schemas: Enterprise clients can add custom fields instantly", "Easy Compliance: 'Delete all data for Company X' is just deleting one actor"].map((item, i) => (
							<motion.li
								key={i}
								initial={{ opacity: 0, x: -20 }}
								whileInView={{ opacity: 1, x: 0 }}
								viewport={{ once: true }}
								transition={{ duration: 0.5, delay: 0.2 + i * 0.1 }}
								className="flex items-center gap-3 text-zinc-300"
							>
								<div className="w-5 h-5 rounded-full bg-violet-500/20 flex items-center justify-center">
									<Check className="w-3 h-3 text-violet-400" />
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
					<div className="absolute inset-0 bg-gradient-to-r from-violet-500/20 to-transparent rounded-2xl blur-2xl" />
					<div className="relative rounded-2xl border border-white/10 bg-zinc-900 p-6 shadow-2xl">
						<div className="flex items-center justify-between mb-6 border-b border-white/5 pb-4">
							<div className="flex items-center gap-3">
								<div className="w-8 h-8 rounded bg-violet-500/20 flex items-center justify-center">
									<Table2 className="w-5 h-5 text-violet-400" />
								</div>
								<div>
									<div className="text-sm font-medium text-white">Tenant: Acme Corp</div>
									<div className="text-xs text-zinc-500">DB Size: 450MB</div>
								</div>
							</div>
							<div className="px-2 py-1 rounded bg-green-500/10 text-green-400 text-xs border border-green-500/20">Online</div>
						</div>
						<div className="space-y-4 text-sm font-mono">
							<div className="p-3 rounded bg-zinc-950 border border-white/5 text-zinc-400">&gt; GET /leads?status=new</div>
							<div className="p-3 rounded bg-violet-900/20 border border-violet-500/30 text-violet-200">&lt; Result: 14,203 objects (Returned in 4ms)</div>
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
			title: "SaaS Platforms",
			desc: "Give every customer their own isolated environment. Scale to millions of tenants effortlessly.",
		},
		{
			title: "Local-First Sync",
			desc: "Serve as the authoritative cloud replica for local state on user devices.",
		},
		{
			title: "User Settings",
			desc: "Store complex user preferences and configurations JSON in a dedicated actor, not a giant shared table.",
		},
		{
			title: "IoT Digital Twins",
			desc: "One actor per device. Store sensor history and configuration state in a dedicated micro-store.",
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
					Built for Scale
				</motion.h2>
				<div className="grid md:grid-cols-2 lg:grid-cols-4 gap-6">
					{cases.map((c, i) => (
						<motion.div
							key={i}
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: i * 0.05 }}
							className="p-6 rounded-xl border border-white/10 bg-zinc-900/30 hover:bg-violet-900/10 hover:border-violet-500/30 transition-colors group"
						>
							<div className="mb-4">
								<Key className="w-6 h-6 text-violet-500 group-hover:scale-110 transition-transform" />
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
				{["Drizzle", "Kysely", "Zod", "Prisma (JSON)", "TypeORM"].map((tech, i) => (
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

export default function PerTenantDBPage() {
	return (
		<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-violet-500/30 selection:text-violet-200">
			<main>
				<Hero />
				<IsolationArchitecture />
				<StateFeatures />
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
							Isolate your data.
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-lg text-zinc-400 mb-10 leading-relaxed"
						>
							Start building multi-tenant applications with the security and performance of single-tenant architecture.
						</motion.p>
						<motion.div
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.2 }}
							className="flex flex-col sm:flex-row items-center justify-center gap-4"
						>
							<button className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black shadow-sm hover:bg-zinc-200 transition-colors">
								Start Building Now
							</button>
							<button className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white shadow-sm hover:border-white/20 transition-colors">
								Read the Docs
							</button>
						</motion.div>
					</div>
				</section>
			</main>
		</div>
	);
}

