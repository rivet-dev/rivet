"use client";

import {
	Zap,
	Globe,
	ArrowRight,
	Database,
	Check,
	RefreshCw,
	Activity,
	Map,
	GitBranch,
	Landmark,
} from "lucide-react";
import { motion } from "framer-motion";

// --- Shared Design Components ---
const Badge = ({ text }: { text: string }) => (
	<div className="inline-flex items-center gap-2 rounded-full border border-white/10 px-3 py-1 text-xs text-zinc-400 mb-6">
		<span className="h-1.5 w-1.5 rounded-full bg-[#FF4500]" />
		{text}
	</div>
);

const CodeBlock = ({ code, fileName = "global.ts" }: { code: string; fileName?: string }) => {
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
			else if (["state", "actions", "broadcast", "c", "region", "data", "sync", "replicate"].includes(trimmed)) {
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

const FeatureItem = ({ title, description, icon: Icon }: { title: string; description: string; icon: typeof Database }) => (
	<div className="border-t border-white/10 pt-6">
		<div className="mb-3 text-zinc-500">
			<Icon className="h-4 w-4" />
		</div>
		<h3 className="text-sm font-normal text-white mb-1">{title}</h3>
		<p className="text-sm leading-relaxed text-zinc-500">{description}</p>
	</div>
);

const Hero = () => (
	<section className="relative pt-32 pb-20 md:pt-48 md:pb-32 overflow-hidden">
		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<Badge text="Multi-Region State" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="text-4xl md:text-6xl font-normal text-white tracking-tight leading-[1.1] mb-6"
					>
						Databases that Span the Globe
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-base text-zinc-500 leading-relaxed mb-8 max-w-lg"
					>
						Stop wrestling with read replicas and eventual consistency. Rivet Actors replicate state automatically across regions, giving every user local latency for both reads and writes.
					</motion.p>

					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="flex flex-col sm:flex-row items-center gap-4"
					>
						<a href="/docs" className="inline-flex items-center justify-center whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200 gap-2">
							Get Started
							<ArrowRight className="w-4 h-4" />
						</a>
					</motion.div>
				</div>

				<div className="flex-1 w-full max-w-xl">
					<CodeBlock
							fileName="global_config.ts"
							code={`import { actor } from "rivetkit";

// Deployed to EU regions for data sovereignty
export const globalStore = actor({
  state: { inventory: {} },

  actions: {
    // Reads are served from the nearest replica
    getItem: (c, id) => c.state.inventory[id],

    // Writes update state atomically
    updateStock: (c, id, qty) => {
      c.state.inventory[id] = qty;
      c.broadcast("stock_update", { id, qty });
    },

    getRegion: (c) => c.region
  }
});`}
						/>
				</div>
			</div>
		</div>
	</section>
);

const GlobalArchitecture = () => {
	return (
		<section className="border-t border-white/10 py-48">
			<div className="max-w-7xl mx-auto px-6">
				<div className="mb-16 text-center">
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-6"
					>
						Latency is the Enemy
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-base leading-relaxed text-zinc-500 max-w-2xl"
					>
						Don't send your users across the ocean for a database query. Rivet brings the data to the edge, synchronizing state changes across regions.
					</motion.p>
				</div>

				<div className="relative h-[600px] w-full rounded-lg border border-white/10 bg-black flex items-center justify-center overflow-hidden p-0 md:p-8">
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
						<circle cx="49" cy="25" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.2s"/></circle>
						<circle cx="48" cy="25" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.3s"/></circle>
						<circle cx="47" cy="25" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.5s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="45" cy="25" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.7s"/></circle>
						<circle cx="43" cy="25" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="51" cy="24" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.9s"/></circle>
						<circle cx="50" cy="24" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="0.4s"/></circle>
						<circle cx="49" cy="24" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.6s"/></circle>
						<circle cx="46" cy="24" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.0s"/></circle>
						<circle cx="44" cy="24" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.7s"/></circle>
						<circle cx="42" cy="24" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.8s"/></circle>
						
						{/* Northeastern Canada */}
						<circle cx="54" cy="19" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.5s"/></circle>
						<circle cx="53" cy="19" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.6s"/></circle>
						<circle cx="51" cy="19" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="0.8s"/></circle>
						<circle cx="49" cy="19" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="2.1s"/></circle>
						<circle cx="47" cy="19" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="44" cy="19" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.3s"/></circle>
						<circle cx="56" cy="18" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.5s" repeatCount="indefinite" begin="1.4s"/></circle>
						<circle cx="55" cy="18" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.7s"/></circle>
						<circle cx="52" cy="18" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.5s"/></circle>
						<circle cx="50" cy="18" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="48" cy="18" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.8s"/></circle>
						
						{/* Caribbean - Blue dots */}
						<circle cx="38" cy="44" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.4s"/></circle>
						<circle cx="41" cy="44" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.5s"/></circle>
						<circle cx="42" cy="44" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.1s"/></circle>
						
						{/* Europe - Blue dots */}
						<circle cx="99" cy="29" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.3s"/></circle>
						<circle cx="95" cy="29" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.5s"/></circle>
						<circle cx="89" cy="28" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.5s"/></circle>
						<circle cx="99" cy="27" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="98" cy="27" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="2.0s"/></circle>
						<circle cx="95" cy="27" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.5s" repeatCount="indefinite" begin="1.4s"/></circle>
						<circle cx="107" cy="28" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.8s"/></circle>
						<circle cx="106" cy="28" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.6s"/></circle>
						<circle cx="109" cy="27" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.2s"/></circle>
						<circle cx="106" cy="27" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.9s"/></circle>
						
						{/* Western North America - Blue dots */}
						<circle cx="26" cy="39" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.4s"/></circle>
						<circle cx="24" cy="39" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="0.8s"/></circle>
						<circle cx="26" cy="38" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="25" cy="38" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="27" cy="37" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.0s"/></circle>
						<circle cx="25" cy="37" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="2.1s"/></circle>
						
						{/* Asia - Blue dots */}
						<circle cx="168" cy="39" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.5s"/></circle>
						<circle cx="165" cy="39" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.6s"/></circle>
						<circle cx="160" cy="39" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.9s"/></circle>
						<circle cx="157" cy="39" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="154" cy="39" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.4s"/></circle>
						<circle cx="150" cy="39" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.1s"/></circle>
						<circle cx="166" cy="38" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.5s" repeatCount="indefinite" begin="0.7s"/></circle>
						<circle cx="162" cy="38" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.8s"/></circle>
						
						{/* Australia - Blue dots */}
						<circle cx="192" cy="89" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.4s"/></circle>
						<circle cx="191" cy="89" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.6s"/></circle>
						<circle cx="193" cy="88" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.9s"/></circle>
						<circle cx="192" cy="88" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="198" cy="85" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="197" cy="85" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.7s"/></circle>
						
						{/* Africa - Blue dots */}
						<circle cx="108" cy="79" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.5s"/></circle>
						<circle cx="106" cy="79" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.6s"/></circle>
						<circle cx="104" cy="79" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="109" cy="78" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="107" cy="78" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="0.7s"/></circle>
						<circle cx="105" cy="78" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.4s"/></circle>
						
						{/* South America - Blue dots */}
						<circle cx="58" cy="77" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0s"/></circle>
						<circle cx="44" cy="76" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.15s"/></circle>
						<circle cx="45" cy="74" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.3s"/></circle>
						<circle cx="53" cy="72" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.45s"/></circle>
						<circle cx="44" cy="71" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="55" cy="69" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.75s"/></circle>
						<circle cx="46" cy="68" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="58" cy="66" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.05s"/></circle>
						<circle cx="50" cy="65" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="65" cy="63" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.35s"/></circle>
						
						{/* More Europe coverage */}
						<circle cx="115" cy="40" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0s"/></circle>
						<circle cx="101" cy="39" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.15s"/></circle>
						<circle cx="108" cy="36" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.45s"/></circle>
						<circle cx="92" cy="33" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.75s"/></circle>
						<circle cx="92" cy="24" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.35s"/></circle>
						
						{/* More Asia coverage - expanded */}
						<circle cx="166" cy="55" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0s"/></circle>
						<circle cx="165" cy="54" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.5s"/></circle>
						<circle cx="157" cy="54" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.0s"/></circle>
						<circle cx="154" cy="48" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.15s"/></circle>
						<circle cx="156" cy="47" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="151" cy="44" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.3s"/></circle>
						<circle cx="143" cy="44" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.4s"/></circle>
						<circle cx="159" cy="41" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.45s"/></circle>
						<circle cx="152" cy="41" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="147" cy="41" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.65s"/></circle>
						<circle cx="142" cy="40" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="137" cy="40" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.3s"/></circle>
						<circle cx="163" cy="39" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.5s"/></circle>
						<circle cx="156" cy="39" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.15s"/></circle>
						<circle cx="149" cy="39" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.35s"/></circle>
						<circle cx="161" cy="38" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.85s"/></circle>
						<circle cx="155" cy="38" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.45s"/></circle>
						<circle cx="145" cy="37" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="138" cy="37" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.7s"/></circle>
						<circle cx="132" cy="36" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.05s"/></circle>
						<circle cx="128" cy="36" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.55s"/></circle>
						<circle cx="161" cy="34" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="155" cy="34" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.25s"/></circle>
						<circle cx="150" cy="33" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.35s"/></circle>
						<circle cx="143" cy="33" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.7s"/></circle>
						<circle cx="139" cy="32" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.5s"/></circle>
						<circle cx="133" cy="32" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.95s"/></circle>
						
						{/* More South America */}
						<circle cx="59" cy="74" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.4s"/></circle>
						<circle cx="49" cy="73" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.1s"/></circle>
						<circle cx="56" cy="71" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.65s"/></circle>
						<circle cx="48" cy="70" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.3s"/></circle>
						<circle cx="54" cy="69" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.35s"/></circle>
						<circle cx="51" cy="67" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="62" cy="65" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.45s"/></circle>
						<circle cx="56" cy="64" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.55s"/></circle>
						<circle cx="63" cy="62" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.15s"/></circle>
						<circle cx="55" cy="61" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.75s"/></circle>
						
						{/* More Africa coverage */}
						<circle cx="105" cy="81" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0s"/></circle>
						<circle cx="102" cy="77" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.15s"/></circle>
						<circle cx="109" cy="74" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.3s"/></circle>
						<circle cx="106" cy="72" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.45s"/></circle>
						<circle cx="105" cy="70" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="109" cy="66" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.9s"/></circle>
						
						{/* More Australia coverage */}
						<circle cx="191" cy="90" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0s"/></circle>
						<circle cx="190" cy="89" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.15s"/></circle>
						<circle cx="194" cy="87" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.3s"/></circle>
						<circle cx="182" cy="84" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="177" cy="84" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.75s"/></circle>
						<circle cx="181" cy="82" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.05s"/></circle>
						
						{/* Central North America */}
						<circle cx="35" cy="35" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0s"/></circle>
						<circle cx="33" cy="33" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.3s"/></circle>
						<circle cx="30" cy="31" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.75s"/></circle>
						<circle cx="33" cy="29" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.2s"/></circle>
						
						{/* Eastern North America - expanded */}
						<circle cx="37" cy="35" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0s"/></circle>
						<circle cx="36" cy="35" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="34" cy="35" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.1s"/></circle>
						<circle cx="39" cy="33" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.15s"/></circle>
						<circle cx="38" cy="33" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="40" cy="32" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.3s"/></circle>
						<circle cx="39" cy="32" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="1.3s"/></circle>
						<circle cx="36" cy="32" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.7s"/></circle>
						<circle cx="41" cy="31" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.45s"/></circle>
						<circle cx="38" cy="31" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.5s"/></circle>
						<circle cx="36" cy="31" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="42" cy="30" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="40" cy="30" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.4s"/></circle>
						<circle cx="39" cy="30" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.6s"/></circle>
						<circle cx="40" cy="29" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.9s"/></circle>
						<circle cx="37" cy="29" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="36" cy="29" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.5s"/></circle>
						<circle cx="42" cy="28" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.05s"/></circle>
						<circle cx="41" cy="28" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0s"/></circle>
						<circle cx="38" cy="28" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.4s"/></circle>
						<circle cx="37" cy="28" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.2s"/></circle>
						<circle cx="43" cy="27" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.35s"/></circle>
						<circle cx="39" cy="27" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.8s"/></circle>
						<circle cx="38" cy="27" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.7s"/></circle>
						<circle cx="42" cy="26" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.35s"/></circle>
						<circle cx="41" cy="26" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="1.25s"/></circle>
						
						{/* Additional West North America */}
						<circle cx="30" cy="45" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0.0s"/></circle>
						<circle cx="25" cy="45" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="27" cy="44" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="22" cy="44" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="23" cy="43" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="23" cy="42" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="23" cy="41" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="23" cy="40" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="26" cy="39" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="21" cy="39" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="24" cy="38" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="19" cy="38" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="24" cy="37" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="19" cy="37" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="26" cy="36" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="21" cy="36" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="30" cy="35" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="24" cy="35" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Additional Africa */}
						<circle cx="103" cy="78" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="100" cy="76" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="101" cy="73" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="110" cy="71" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="104" cy="70" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="101" cy="69" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="98" cy="68" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="113" cy="66" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="106" cy="65" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="99" cy="64" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="109" cy="62" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="101" cy="61" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="114" cy="59" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="110" cy="58" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="108" cy="57" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="107" cy="56" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Additional Asia */}
						<circle cx="180" cy="60" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0.0s"/></circle>
						<circle cx="167" cy="57" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="143" cy="52" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="159" cy="47" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="168" cy="44" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="154" cy="42" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="139" cy="40" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="135" cy="39" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="129" cy="38" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="130" cy="37" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="131" cy="36" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="132" cy="35" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="134" cy="34" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="137" cy="33" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="141" cy="32" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="147" cy="31" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="151" cy="30" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Additional Latin America */}
						<circle cx="56" cy="80" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0.0s"/></circle>
						<circle cx="49" cy="79" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="57" cy="77" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="51" cy="76" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="44" cy="75" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="57" cy="73" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="55" cy="72" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="54" cy="71" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="53" cy="70" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="52" cy="69" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="51" cy="68" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="50" cy="67" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="50" cy="66" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="50" cy="65" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="51" cy="64" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="53" cy="63" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="55" cy="62" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="56" cy="61" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Additional Europe */}
						<circle cx="120" cy="45" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0.0s"/></circle>
						<circle cx="109" cy="44" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="100" cy="43" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="92" cy="42" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="119" cy="40" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="109" cy="39" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="101" cy="38" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="93" cy="37" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="86" cy="36" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="115" cy="34" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="119" cy="32" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="116" cy="30" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="118" cy="28" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="117" cy="26" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="91" cy="25" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="104" cy="23" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="117" cy="21" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="100" cy="20" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Additional Australia */}
						<circle cx="192" cy="89" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="193" cy="88" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="177" cy="88" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="178" cy="87" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="179" cy="85" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="178" cy="84" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="182" cy="83" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="179" cy="83" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="183" cy="82" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="180" cy="82" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="177" cy="82" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="183" cy="81" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="180" cy="81" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="177" cy="81" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="186" cy="80" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Alaska */}
						<circle cx="35" cy="30" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0.0s"/></circle>
						<circle cx="18" cy="30" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="22" cy="29" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="26" cy="28" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="30" cy="27" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="33" cy="26" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="16" cy="26" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="20" cy="25" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="23" cy="24" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="25" cy="23" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="27" cy="22" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="28" cy="21" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="29" cy="20" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="30" cy="19" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="31" cy="18" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="32" cy="17" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="34" cy="16" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="17" cy="16" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Western Canada */}
						<circle cx="42" cy="30" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0.0s"/></circle>
						<circle cx="36" cy="30" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="37" cy="29" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="38" cy="28" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="43" cy="27" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="46" cy="26" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="40" cy="26" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="47" cy="25" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="37" cy="25" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="46" cy="24" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="37" cy="24" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="46" cy="23" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="40" cy="23" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="45" cy="22" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="39" cy="22" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="48" cy="21" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="42" cy="21" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="36" cy="21" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Western USA */}
						<circle cx="32" cy="42" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0.0s"/></circle>
						<circle cx="24" cy="41" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="21" cy="40" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="21" cy="39" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="21" cy="38" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="23" cy="37" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="27" cy="36" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="19" cy="36" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="26" cy="35" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="18" cy="35" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="26" cy="34" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="18" cy="34" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="26" cy="33" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="18" cy="33" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="27" cy="32" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="19" cy="32" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="29" cy="31" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="21" cy="31" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
						
						{/* Northern South America */}
						<circle cx="57" cy="75" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="0.0s"/></circle>
						<circle cx="59" cy="74" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="0.12s"/></circle>
						<circle cx="45" cy="74" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="0.24s"/></circle>
						<circle cx="47" cy="73" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="0.36s"/></circle>
						<circle cx="50" cy="72" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="0.48s"/></circle>
						<circle cx="53" cy="71" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="0.6s"/></circle>
						<circle cx="56" cy="70" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="0.72s"/></circle>
						<circle cx="59" cy="69" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="0.84s"/></circle>
						<circle cx="45" cy="69" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.3s" repeatCount="indefinite" begin="0.96s"/></circle>
						<circle cx="48" cy="68" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.4s" repeatCount="indefinite" begin="1.08s"/></circle>
						<circle cx="51" cy="67" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.5s" repeatCount="indefinite" begin="1.2s"/></circle>
						<circle cx="54" cy="66" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.6s" repeatCount="indefinite" begin="1.32s"/></circle>
						<circle cx="57" cy="65" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.7s" repeatCount="indefinite" begin="1.44s"/></circle>
						<circle cx="60" cy="64" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.8s" repeatCount="indefinite" begin="1.56s"/></circle>
						<circle cx="46" cy="64" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="2.9s" repeatCount="indefinite" begin="1.68s"/></circle>
						<circle cx="49" cy="63" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.0s" repeatCount="indefinite" begin="1.8s"/></circle>
						<circle cx="52" cy="62" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.1s" repeatCount="indefinite" begin="1.92s"/></circle>
						<circle cx="55" cy="61" r=".3" fill="#FF4500" opacity="0"><animate attributeName="opacity" values="0;1;0" dur="3.2s" repeatCount="indefinite" begin="2.04s"/></circle>
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
		},
		{
			title: "Write Locally",
			description: "Writes are accepted by the local node and asynchronously replicated to other regions. No global locking required for most ops.",
			icon: Zap,
		},
		{
			title: "Conflict Resolution",
			description: "Built-in CRDT (Conflict-free Replicated Data Types) support handles concurrent edits gracefully.",
			icon: GitBranch,
		},
		{
			title: "Edge Caching",
			description: "Actors act as intelligent caches that can execute logic. Invalidate cache globally with a single broadcast.",
			icon: RefreshCw,
		},
		{
			title: "Data Sovereignty",
			description: "Pin specific actors to specific regions. Ensure data never leaves a jurisdiction (e.g. Germany) to satisfy compliance.",
			icon: Landmark,
		},
		{
			title: "Partition Tolerance",
			description: "If a region goes dark, the rest of the cluster continues to operate. State heals automatically when connectivity returns.",
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
					className="mb-20"
				>
					<h2 className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-2">Distributed Superpowers</h2>
					<p className="text-zinc-500 text-lg leading-relaxed">The benefits of a global CDN, but for your application state.</p>
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
							<FeatureItem {...feat} />
						</motion.div>
					))}
				</div>
			</div>
		</section>
	);
};

const CaseStudy = () => (
	<section className="border-t border-white/10 py-48">
		<div className="max-w-7xl mx-auto px-6">
			<div className="grid md:grid-cols-2 gap-16 items-center">
				<div>
					<Badge text="Case Study" />
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-2"
					>
						Global Inventory System
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-base text-zinc-500 mb-8 leading-relaxed"
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
								<div className="w-5 h-5 rounded-full bg-white/10 flex items-center justify-center">
									<Check className="w-3 h-3 text-white" />
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
					<div className="relative rounded-lg border border-white/10 bg-black p-6 shadow-2xl">
						<div className="flex items-center justify-between mb-6 border-b border-white/10 pb-4">
							<div className="flex items-center gap-3">
								<div className="w-8 h-8 rounded bg-white/10 flex items-center justify-center">
									<Globe className="w-5 h-5 text-white" />
								</div>
								<div>
									<div className="text-sm font-medium text-white">Item: SKU-992</div>
									<div className="text-xs text-zinc-500">Global Stock: 14,200</div>
								</div>
							</div>
							<div className="px-2 py-1 rounded bg-green-500/10 text-green-400 text-xs border border-green-500/20">Synced</div>
						</div>
						<div className="space-y-4 text-sm font-mono">
							<div className="p-3 rounded bg-zinc-950 border border-white/10 text-zinc-400 flex justify-between">
								<span>New York</span>
								<span className="text-white">4,200</span>
							</div>
							<div className="p-3 rounded bg-zinc-950 border border-white/10 text-zinc-400 flex justify-between">
								<span>London</span>
								<span className="text-white">3,800</span>
							</div>
							<div className="p-3 rounded bg-zinc-950 border border-white/10 text-zinc-400 flex justify-between">
								<span>Tokyo</span>
								<span className="text-white">6,200</span>
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
		<section className="border-t border-white/10 py-48">
			<div className="max-w-7xl mx-auto px-6">
				<motion.h2
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-12 text-center"
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
							className="border-t border-white/10 pt-6"
						>
							<div className="mb-3 text-zinc-500">
								<Map className="h-4 w-4" />
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
		<div className="max-w-7xl mx-auto px-6 relative z-10 text-center">
			<motion.h2
				initial={{ opacity: 0, y: 20 }}
				whileInView={{ opacity: 1, y: 0 }}
				viewport={{ once: true }}
				transition={{ duration: 0.5 }}
				className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-12"
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
						className="px-2 py-1 rounded-md border border-white/5 bg-black/50 text-zinc-400 text-xs font-mono hover:text-white hover:border-white/30 transition-colors cursor-default backdrop-blur-sm"
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
		<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-[#FF4500]/30 selection:text-orange-200">
			<main>
				<Hero />
				<GlobalArchitecture />
				<ReplicationFeatures />
				<CaseStudy />
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
							className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-6"
						>
							Go Global.
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-base text-zinc-500 mb-10 leading-relaxed"
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
							<a href="/docs" className="inline-flex items-center justify-center whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200">
								Start Building Now
							</a>
							<a href="/docs/actors/state" className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20">
								Read the Docs
							</a>
						</motion.div>
					</div>
				</section>
			</main>
		</div>
	);
}
