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
	Cpu,
	RefreshCw,
	Clock,
	Shield,
	Cloud,
	Activity,
	Server,
	Map,
	GitBranch,
	Landmark,
} from "lucide-react";
import { motion } from "framer-motion";

// --- Shared Design Components ---
const Badge = ({ text, color = "blue" }) => {
	const colorClasses = {
		orange: "text-orange-400 border-orange-500/20 bg-orange-500/10",
		blue: "text-blue-400 border-blue-500/20 bg-blue-500/10",
		red: "text-red-400 border-red-500/20 bg-red-500/10",
		zinc: "text-zinc-400 border-zinc-500/20 bg-zinc-500/10",
		purple: "text-purple-400 border-purple-500/20 bg-purple-500/10",
	};

	return (
		<div
			className={`inline-flex items-center gap-2 px-3 py-1 rounded-full border text-xs font-medium mb-8 transition-colors cursor-default ${colorClasses[color]}`}
		>
			<span className={`w-1.5 h-1.5 rounded-full ${color === "orange" ? "bg-orange-400" : color === "blue" ? "bg-blue-400" : color === "red" ? "bg-red-400" : color === "purple" ? "bg-purple-400" : "bg-zinc-400"} animate-pulse`} />
			{text}
		</div>
	);
};

const CodeBlock = ({ code, fileName = "global.ts" }) => {
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
											} else if (["actor", "broadcast", "getItem", "updateStock"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-400">{part}</span>);
											} else if (["state", "actions", "inventory", "region", "id", "qty"].includes(trimmed)) {
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

const SolutionCard = ({ title, description, icon: Icon, color = "blue" }) => {
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
			case "purple":
				return {
					bg: "bg-purple-500/10",
					text: "text-purple-400",
					hoverBg: "group-hover:bg-purple-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(168,85,247,0.5)]",
					border: "border-purple-500",
					glow: "rgba(168,85,247,0.15)",
				};
			default:
				return {
					bg: "bg-blue-500/10",
					text: "text-blue-400",
					hoverBg: "group-hover:bg-blue-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(59,130,246,0.5)]",
					border: "border-blue-500",
					glow: "rgba(59,130,246,0.15)",
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
		<div className="absolute top-0 left-1/2 -translate-x-1/2 w-[1000px] h-[500px] bg-blue-500/[0.03] blur-[100px] rounded-full pointer-events-none" />

		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<Badge text="Multi-Region State" color="blue" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="text-5xl md:text-7xl font-medium text-white tracking-tight leading-[1.1] mb-6"
					>
						Databases that <br />
						<span className="text-blue-400">Span the Globe.</span>
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg md:text-xl text-zinc-400 leading-relaxed mb-8 max-w-lg"
					>
						Stop wrestling with read replicas and eventual consistency. Rivet Actors replicate state automatically across regions, giving every user local latency for both reads and writes.
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
						<div className="absolute -inset-1 bg-gradient-to-r from-blue-500/20 to-cyan-500/20 rounded-xl blur opacity-40" />
						<CodeBlock
							fileName="global_config.ts"
							code={`import { actor } from "rivetkit";

export const globalStore = actor({
  // Pin data to specific regions for sovereignty
  // This actor never leaves the EU
  region: ["fra", "lhr"],
  
  state: { inventory: {} },

  actions: {
    // Reads are served from the nearest local replica
    getItem: (c, id) => c.state.inventory[id],

    // Writes are coordinated globally (Paxos/Raft)
    updateStock: (c, id, qty) => {
      c.state.inventory[id] = qty;
      c.broadcast("stock_update", { id, qty });
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

const GlobalArchitecture = () => {
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
						Latency is the Enemy
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-zinc-400 max-w-2xl mx-auto text-lg leading-relaxed"
					>
						Don't send your users across the ocean for a database query. Rivet brings the data to the edge, synchronizing state changes across regions.
					</motion.p>
				</div>

				<div className="relative h-[600px] w-full rounded-2xl border border-white/10 bg-zinc-900/20 flex items-center justify-center overflow-hidden p-0 md:p-8">
					{/* World Map Background - static only */}
					<img 
						src="/solutions/world-map.svg" 
						alt="World Map"
						className="absolute inset-0 w-full h-full object-contain opacity-40 pointer-events-none"
					/>

					{/* Animated Activity Dots - full brightness overlay */}
					<svg 
						xmlns="http://www.w3.org/2000/svg" 
						viewBox="0 0 201 97" 
						className="absolute inset-0 w-full h-full pointer-events-none"
						preserveAspectRatio="xMidYMid meet"
					>
						{/* Eastern Canada - on actual map dots */}
						<circle cx="49" cy="25" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.2s"/></circle>
						<circle cx="48" cy="25" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.3s"/></circle>
						<circle cx="47" cy="25" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.5s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="45" cy="25" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.7s"/></circle>
						<circle cx="43" cy="25" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="51" cy="24" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.9s"/></circle>
						<circle cx="50" cy="24" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="0.4s"/></circle>
						<circle cx="49" cy="24" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.6s"/></circle>
						<circle cx="46" cy="24" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.0s"/></circle>
						<circle cx="44" cy="24" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.7s"/></circle>
						<circle cx="42" cy="24" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.8s"/></circle>
						
						{/* Northeastern Canada */}
						<circle cx="54" cy="19" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.5s"/></circle>
						<circle cx="53" cy="19" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.6s"/></circle>
						<circle cx="51" cy="19" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="0.8s"/></circle>
						<circle cx="49" cy="19" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="2.1s"/></circle>
						<circle cx="47" cy="19" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="44" cy="19" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.3s"/></circle>
						<circle cx="56" cy="18" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.5s" repeatCount="indefinite" begin="1.4s"/></circle>
						<circle cx="55" cy="18" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.7s"/></circle>
						<circle cx="52" cy="18" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.5s"/></circle>
						<circle cx="50" cy="18" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="48" cy="18" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.8s"/></circle>
						
						{/* Caribbean - Blue dots */}
						<circle cx="38" cy="44" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.4s"/></circle>
						<circle cx="41" cy="44" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.5s"/></circle>
						<circle cx="42" cy="44" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.1s"/></circle>
						
						{/* Europe - Blue dots */}
						<circle cx="99" cy="29" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.3s"/></circle>
						<circle cx="95" cy="29" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.5s"/></circle>
						<circle cx="89" cy="28" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.5s"/></circle>
						<circle cx="99" cy="27" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="98" cy="27" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="2.0s"/></circle>
						<circle cx="95" cy="27" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.5s" repeatCount="indefinite" begin="1.4s"/></circle>
						<circle cx="107" cy="28" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.8s"/></circle>
						<circle cx="106" cy="28" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.6s"/></circle>
						<circle cx="109" cy="27" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.2s"/></circle>
						<circle cx="106" cy="27" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.9s"/></circle>
						
						{/* Western North America - Blue dots */}
						<circle cx="26" cy="39" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.4s"/></circle>
						<circle cx="24" cy="39" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="0.8s"/></circle>
						<circle cx="26" cy="38" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="25" cy="38" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="27" cy="37" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.0s"/></circle>
						<circle cx="25" cy="37" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="2.1s"/></circle>
						
						{/* Asia - Blue dots */}
						<circle cx="168" cy="39" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.5s"/></circle>
						<circle cx="165" cy="39" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.6s"/></circle>
						<circle cx="160" cy="39" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.9s"/></circle>
						<circle cx="157" cy="39" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="154" cy="39" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.4s"/></circle>
						<circle cx="150" cy="39" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.1s"/></circle>
						<circle cx="166" cy="38" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.5s" repeatCount="indefinite" begin="0.7s"/></circle>
						<circle cx="162" cy="38" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.8s"/></circle>
						
						{/* Australia - Blue dots */}
						<circle cx="192" cy="89" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.4s"/></circle>
						<circle cx="191" cy="89" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.6s"/></circle>
						<circle cx="193" cy="88" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.9s"/></circle>
						<circle cx="192" cy="88" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="198" cy="85" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="197" cy="85" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.7s"/></circle>
						
						{/* Africa - Blue dots */}
						<circle cx="108" cy="79" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.5s"/></circle>
						<circle cx="106" cy="79" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.6s"/></circle>
						<circle cx="104" cy="79" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="109" cy="78" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="107" cy="78" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="0.7s"/></circle>
						<circle cx="105" cy="78" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.4s"/></circle>
						
						{/* South America - Blue dots */}
						<circle cx="58" cy="77" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0s"/></circle>
						<circle cx="44" cy="76" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.15s"/></circle>
						<circle cx="45" cy="74" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.3s"/></circle>
						<circle cx="53" cy="72" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.45s"/></circle>
						<circle cx="44" cy="71" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="55" cy="69" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.75s"/></circle>
						<circle cx="46" cy="68" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="58" cy="66" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.05s"/></circle>
						<circle cx="50" cy="65" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="65" cy="63" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.35s"/></circle>
						
						{/* More Europe coverage */}
						<circle cx="115" cy="40" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0s"/></circle>
						<circle cx="101" cy="39" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.15s"/></circle>
						<circle cx="108" cy="36" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.45s"/></circle>
						<circle cx="92" cy="33" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.75s"/></circle>
						<circle cx="92" cy="24" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.35s"/></circle>
						
						{/* More Asia coverage - expanded */}
						<circle cx="166" cy="55" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0s"/></circle>
						<circle cx="165" cy="54" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.5s"/></circle>
						<circle cx="157" cy="54" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.0s"/></circle>
						<circle cx="154" cy="48" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.15s"/></circle>
						<circle cx="156" cy="47" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="151" cy="44" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.3s"/></circle>
						<circle cx="143" cy="44" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.4s"/></circle>
						<circle cx="159" cy="41" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.45s"/></circle>
						<circle cx="152" cy="41" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="147" cy="41" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.65s"/></circle>
						<circle cx="142" cy="40" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="137" cy="40" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.3s"/></circle>
						<circle cx="163" cy="39" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.5s"/></circle>
						<circle cx="156" cy="39" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.15s"/></circle>
						<circle cx="149" cy="39" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.35s"/></circle>
						<circle cx="161" cy="38" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.85s"/></circle>
						<circle cx="155" cy="38" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.45s"/></circle>
						<circle cx="145" cy="37" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="138" cy="37" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.7s"/></circle>
						<circle cx="132" cy="36" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.05s"/></circle>
						<circle cx="128" cy="36" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.55s"/></circle>
						<circle cx="161" cy="34" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="155" cy="34" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.25s"/></circle>
						<circle cx="150" cy="33" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.35s"/></circle>
						<circle cx="143" cy="33" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.7s"/></circle>
						<circle cx="139" cy="32" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.5s"/></circle>
						<circle cx="133" cy="32" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.95s"/></circle>
						
						{/* More South America */}
						<circle cx="59" cy="74" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.4s"/></circle>
						<circle cx="49" cy="73" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.1s"/></circle>
						<circle cx="56" cy="71" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.65s"/></circle>
						<circle cx="48" cy="70" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.3s"/></circle>
						<circle cx="54" cy="69" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.35s"/></circle>
						<circle cx="51" cy="67" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="62" cy="65" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.45s"/></circle>
						<circle cx="56" cy="64" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.55s"/></circle>
						<circle cx="63" cy="62" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.15s"/></circle>
						<circle cx="55" cy="61" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.75s"/></circle>
						
						{/* More Africa coverage */}
						<circle cx="105" cy="81" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0s"/></circle>
						<circle cx="102" cy="77" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.15s"/></circle>
						<circle cx="109" cy="74" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.3s"/></circle>
						<circle cx="106" cy="72" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.45s"/></circle>
						<circle cx="105" cy="70" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="109" cy="66" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.9s"/></circle>
						
						{/* More Australia coverage */}
						<circle cx="191" cy="90" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0s"/></circle>
						<circle cx="190" cy="89" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.15s"/></circle>
						<circle cx="194" cy="87" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.3s"/></circle>
						<circle cx="182" cy="84" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="177" cy="84" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.75s"/></circle>
						<circle cx="181" cy="82" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.05s"/></circle>
						
						{/* Central North America */}
						<circle cx="35" cy="35" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0s"/></circle>
						<circle cx="33" cy="33" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.3s"/></circle>
						<circle cx="30" cy="31" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.75s"/></circle>
						<circle cx="33" cy="29" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.2s"/></circle>
						
						{/* Eastern North America - expanded */}
						<circle cx="37" cy="35" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0s"/></circle>
						<circle cx="36" cy="35" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="34" cy="35" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.1s"/></circle>
						<circle cx="39" cy="33" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.15s"/></circle>
						<circle cx="38" cy="33" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="40" cy="32" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.3s"/></circle>
						<circle cx="39" cy="32" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.3s"/></circle>
						<circle cx="36" cy="32" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.7s"/></circle>
						<circle cx="41" cy="31" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.45s"/></circle>
						<circle cx="38" cy="31" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.5s"/></circle>
						<circle cx="36" cy="31" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="42" cy="30" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="40" cy="30" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.4s"/></circle>
						<circle cx="39" cy="30" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.6s"/></circle>
						<circle cx="40" cy="29" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="37" cy="29" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="36" cy="29" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.5s"/></circle>
						<circle cx="42" cy="28" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.05s"/></circle>
						<circle cx="41" cy="28" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0s"/></circle>
						<circle cx="38" cy="28" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.4s"/></circle>
						<circle cx="37" cy="28" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.2s"/></circle>
						<circle cx="43" cy="27" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.35s"/></circle>
						<circle cx="39" cy="27" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.8s"/></circle>
						<circle cx="38" cy="27" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.7s"/></circle>
						<circle cx="42" cy="26" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.35s"/></circle>
						<circle cx="41" cy="26" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.25s"/></circle>
						
						{/* Additional West North America */}
						<circle cx="30" cy="45" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0.0s"/></circle>
						<circle cx="25" cy="45" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="27" cy="44" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="22" cy="44" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="23" cy="43" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="23" cy="42" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="23" cy="41" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="23" cy="40" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="26" cy="39" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="21" cy="39" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="24" cy="38" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="19" cy="38" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="24" cy="37" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="19" cy="37" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="26" cy="36" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="21" cy="36" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="30" cy="35" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="24" cy="35" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Additional Africa */}
						<circle cx="103" cy="78" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="100" cy="76" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="101" cy="73" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="110" cy="71" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="104" cy="70" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="101" cy="69" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="98" cy="68" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="113" cy="66" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="106" cy="65" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="99" cy="64" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="109" cy="62" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="101" cy="61" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="114" cy="59" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="110" cy="58" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="108" cy="57" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="107" cy="56" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Additional Asia */}
						<circle cx="180" cy="60" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0.0s"/></circle>
						<circle cx="167" cy="57" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="143" cy="52" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="159" cy="47" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="168" cy="44" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="154" cy="42" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="139" cy="40" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="135" cy="39" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="129" cy="38" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="130" cy="37" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="131" cy="36" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="132" cy="35" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="134" cy="34" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="137" cy="33" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="141" cy="32" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="147" cy="31" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="151" cy="30" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Additional Latin America */}
						<circle cx="56" cy="80" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0.0s"/></circle>
						<circle cx="49" cy="79" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="57" cy="77" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="51" cy="76" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="44" cy="75" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="57" cy="73" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="55" cy="72" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="54" cy="71" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="53" cy="70" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="52" cy="69" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="51" cy="68" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="50" cy="67" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="50" cy="66" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="50" cy="65" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="51" cy="64" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="53" cy="63" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="55" cy="62" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="56" cy="61" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Additional Europe */}
						<circle cx="120" cy="45" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0.0s"/></circle>
						<circle cx="109" cy="44" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="100" cy="43" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="92" cy="42" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="119" cy="40" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="109" cy="39" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="101" cy="38" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="93" cy="37" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="86" cy="36" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="115" cy="34" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="119" cy="32" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="116" cy="30" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="118" cy="28" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="117" cy="26" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="91" cy="25" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="104" cy="23" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="117" cy="21" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="100" cy="20" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Additional Australia */}
						<circle cx="192" cy="89" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="193" cy="88" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="177" cy="88" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="178" cy="87" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="179" cy="85" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="178" cy="84" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="182" cy="83" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="179" cy="83" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="183" cy="82" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="180" cy="82" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="177" cy="82" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="183" cy="81" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="180" cy="81" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="177" cy="81" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="186" cy="80" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Alaska */}
						<circle cx="35" cy="30" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0.0s"/></circle>
						<circle cx="18" cy="30" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="22" cy="29" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="26" cy="28" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="30" cy="27" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="33" cy="26" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="16" cy="26" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="20" cy="25" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="23" cy="24" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="25" cy="23" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="27" cy="22" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="28" cy="21" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="29" cy="20" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="30" cy="19" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="31" cy="18" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="32" cy="17" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="34" cy="16" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="17" cy="16" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Western Canada */}
						<circle cx="42" cy="30" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0.0s"/></circle>
						<circle cx="36" cy="30" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="37" cy="29" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="38" cy="28" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="43" cy="27" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="46" cy="26" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="40" cy="26" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="47" cy="25" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="37" cy="25" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="46" cy="24" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="37" cy="24" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="46" cy="23" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="40" cy="23" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="45" cy="22" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="39" cy="22" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="48" cy="21" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="42" cy="21" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="36" cy="21" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Western USA */}
						<circle cx="32" cy="42" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0.0s"/></circle>
						<circle cx="24" cy="41" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="21" cy="40" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="21" cy="39" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="21" cy="38" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="23" cy="37" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="27" cy="36" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="19" cy="36" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="26" cy="35" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="18" cy="35" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="26" cy="34" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="18" cy="34" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="26" cy="33" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="18" cy="33" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="27" cy="32" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="19" cy="32" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="29" cy="31" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="21" cy="31" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Northern South America */}
						<circle cx="57" cy="75" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0.0s"/></circle>
						<circle cx="59" cy="74" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="45" cy="74" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="47" cy="73" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="50" cy="72" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="53" cy="71" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="56" cy="70" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="59" cy="69" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="45" cy="69" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="48" cy="68" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="51" cy="67" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="54" cy="66" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="57" cy="65" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="60" cy="64" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="46" cy="64" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="49" cy="63" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="52" cy="62" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="55" cy="61" r=".3" fill="#3b82f6" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
					</svg>
				</div>
			</div>
		</section>
	);
};

const ReplicationFeatures = () => {
	const features = [
		{
			title: "Read from Anywhere",
			description: "Requests are automatically routed to the nearest regional replica. Reads are served from local memory in <5ms.",
			icon: Globe,
			color: "blue",
		},
		{
			title: "Write Locally",
			description: "Writes are accepted by the local node and asynchronously replicated to other regions. No global locking required for most ops.",
			icon: Zap,
			color: "orange",
		},
		{
			title: "Conflict Resolution",
			description: "Built-in CRDT (Conflict-free Replicated Data Types) support handles concurrent edits gracefully.",
			icon: GitBranch,
			color: "purple",
		},
		{
			title: "Edge Caching",
			description: "Actors act as intelligent caches that can execute logic. Invalidate cache globally with a single broadcast.",
			icon: RefreshCw,
			color: "zinc",
		},
		{
			title: "Data Sovereignty",
			description: "Pin specific actors to specific regions. Ensure data never leaves a jurisdiction (e.g. Germany) to satisfy compliance.",
			icon: Landmark,
			color: "red",
		},
		{
			title: "Partition Tolerance",
			description: "If a region goes dark, the rest of the cluster continues to operate. State heals automatically when connectivity returns.",
			icon: Activity,
			color: "blue",
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
					<h2 className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight">Distributed Superpowers</h2>
					<p className="text-zinc-400 text-lg leading-relaxed">The benefits of a global CDN, but for your application state.</p>
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
					<Badge text="Case Study" color="blue" />
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						Global Inventory System
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg text-zinc-400 mb-8 leading-relaxed"
					>
						A retail platform tracking stock across warehouses in 12 countries.
					</motion.p>
					<ul className="space-y-4">
						{["Real-time: Stock levels update globally in <100ms", "Resilient: Local warehouses can continue operating if the transatlantic cable creates latency", "Consistent: Overselling prevented via regional allocation pools"].map((item, i) => (
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
									<Globe className="w-5 h-5 text-blue-400" />
								</div>
								<div>
									<div className="text-sm font-medium text-white">Item: SKU-992</div>
									<div className="text-xs text-zinc-500">Global Stock: 14,200</div>
								</div>
							</div>
							<div className="px-2 py-1 rounded bg-green-500/10 text-green-400 text-xs border border-green-500/20">Synced</div>
						</div>
						<div className="space-y-4 text-sm font-mono">
							<div className="p-3 rounded bg-zinc-950 border border-white/5 text-zinc-400 flex justify-between">
								<span>New York</span>
								<span className="text-blue-400">4,200</span>
							</div>
							<div className="p-3 rounded bg-zinc-950 border border-white/5 text-zinc-400 flex justify-between">
								<span>London</span>
								<span className="text-blue-400">3,800</span>
							</div>
							<div className="p-3 rounded bg-zinc-950 border border-white/5 text-zinc-400 flex justify-between">
								<span>Tokyo</span>
								<span className="text-blue-400">6,200</span>
							</div>
							<div className="text-right text-[10px] text-zinc-500">Replication Lag: 45ms</div>
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
			title: "Global User Profiles",
			desc: "Store user settings and session data close to where they are. Login in Tokyo feels as fast as login in NY.",
		},
		{
			title: "Content Delivery",
			desc: "Serve dynamic, personalized content from the edge without hitting a central origin database.",
		},
		{
			title: "IoT Data Aggregation",
			desc: "Ingest sensor data locally in each region, aggregate it, and replicate summaries to HQ.",
		},
		{
			title: "Multi-Region Failover",
			desc: "Keep a hot standby of your entire application state in a second region for instant disaster recovery.",
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
							className="p-6 rounded-xl border border-white/10 bg-zinc-900/30 hover:bg-blue-900/10 hover:border-blue-500/30 transition-colors group"
						>
							<div className="mb-4">
								<Map className="w-6 h-6 text-blue-500 group-hover:scale-110 transition-transform" />
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
				{["AWS (Global)", "GCP", "Azure", "Fly.io", "Cloudflare", "Vercel Edge"].map((tech, i) => (
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

export default function GeoDistributedDBPage() {
	return (
		<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-blue-500/30 selection:text-blue-200">
			<main>
				<Hero />
				<GlobalArchitecture />
				<ReplicationFeatures />
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
							Go Global.
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-lg text-zinc-400 mb-10 leading-relaxed"
						>
							Stop worrying about latency. Start building applications that feel local to everyone, everywhere.
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

