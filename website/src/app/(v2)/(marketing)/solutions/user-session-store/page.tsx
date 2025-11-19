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
	Calendar,
	GitBranch,
	Timer,
	Mail,
	CreditCard,
	Bell,
	Server,
	Sword,
	Trophy,
	Target,
	Fingerprint,
	Cookie,
	Lock,
	Smartphone,
	ShoppingCart,
	Shield,
} from "lucide-react";
import { motion } from "framer-motion";

// --- Shared Design Components ---
const Badge = ({ text, color = "cyan" }) => {
	const colorClasses = {
		orange: "text-orange-400 border-orange-500/20 bg-orange-500/10",
		blue: "text-blue-400 border-blue-500/20 bg-blue-500/10",
		purple: "text-purple-400 border-purple-500/20 bg-purple-500/10",
		emerald: "text-emerald-400 border-emerald-500/20 bg-emerald-500/10",
		red: "text-red-400 border-red-500/20 bg-red-500/10",
		cyan: "text-cyan-400 border-cyan-500/20 bg-cyan-500/10",
	};

	return (
		<div
			className={`inline-flex items-center gap-2 px-3 py-1 rounded-full border text-xs font-medium mb-8 transition-colors cursor-default ${colorClasses[color]}`}
		>
			<span className={`w-1.5 h-1.5 rounded-full ${color === "orange" ? "bg-orange-400" : color === "blue" ? "bg-blue-400" : color === "purple" ? "bg-purple-400" : color === "emerald" ? "bg-emerald-400" : color === "red" ? "bg-red-400" : "bg-cyan-400"} animate-pulse`} />
			{text}
		</div>
	);
};

const CodeBlock = ({ code, fileName = "session.ts" }) => {
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
			<div className="p-4 overflow-x-auto scrollbar-hide bg-black">
				<pre className="text-sm font-mono leading-relaxed text-zinc-300 bg-black">
					<code className="bg-black">
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
											if (["import", "from", "export", "const", "return", "async", "await", "function", "let", "var", "if", "else"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-purple-400">{part}</span>);
											}
											// Functions & Special Rivet Terms
											else if (["actor", "broadcast", "spawn", "rpc", "schedule", "token", "generate"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-400">{part}</span>);
											}
											// Object Keys / Properties / Methods
											else if (["state", "actions", "user", "cart", "preferences", "theme", "login", "userData", "addToCart", "item", "push", "destroy"].includes(trimmed)) {
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

// --- Refined Session Card matching landing page style with color highlights ---
const SolutionCard = ({ title, description, icon: Icon, color = "cyan" }) => {
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
			case "red":
				return {
					bg: "bg-red-500/10",
					text: "text-red-400",
					hoverBg: "group-hover:bg-red-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(239,68,68,0.5)]",
					border: "border-red-500",
					glow: "rgba(239,68,68,0.15)",
				};
			case "cyan":
				return {
					bg: "bg-cyan-500/10",
					text: "text-cyan-400",
					hoverBg: "group-hover:bg-cyan-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(34,211,238,0.5)]",
					border: "border-cyan-500",
					glow: "rgba(34,211,238,0.15)",
				};
			default:
				return {
					bg: "bg-cyan-500/10",
					text: "text-cyan-400",
					hoverBg: "group-hover:bg-cyan-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(34,211,238,0.5)]",
					border: "border-cyan-500",
					glow: "rgba(34,211,238,0.15)",
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
		<div className="absolute top-0 left-1/2 -translate-x-1/2 w-[1000px] h-[500px] bg-cyan-500/[0.03] blur-[100px] rounded-full pointer-events-none" />

		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<Badge text="Universal Session Layer" color="cyan" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="text-5xl md:text-7xl font-medium text-white tracking-tight leading-[1.1] mb-6"
					>
						Session State. <br />
						<span className="text-cyan-400">Instantly Available.</span>
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg md:text-xl text-zinc-400 leading-relaxed mb-8 max-w-lg"
					>
						Replace Redis and cookies with persistent Actors. Attach rich, real-time state to every user, instantly accessible from the edge without database latency.
					</motion.p>
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="flex flex-col sm:flex-row items-center gap-4"
					>
						<button className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black shadow-sm hover:bg-zinc-200 transition-colors gap-2">
							Create Session
							<ArrowRight className="w-4 h-4" />
						</button>
					</motion.div>
				</div>
				<div className="flex-1 w-full max-w-xl">
					<div className="relative">
						<div className="absolute -inset-1 bg-gradient-to-r from-cyan-500/20 to-blue-500/20 rounded-xl blur opacity-40" />
						<CodeBlock
							fileName="user_session.ts"
							code={`import { actor } from "rivetkit";

export const userSession = actor({
  state: { 
    user: null, 
    cart: [], 
    preferences: { theme: 'dark' } 
  },

  actions: {
    login: (c, userData) => {
      c.state.user = userData;
      // Set expiry for inactivity (30 mins)
      c.schedule("destroy", "30m");
      return c.token.generate();
    },

    addToCart: (c, item) => {
      c.state.cart.push(item);
      // Broadcast update to all user's tabs
      c.broadcast("cart_updated", c.state.cart);
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

const LatencyArchitecture = () => {
	const [step, setStep] = useState(0);

	useEffect(() => {
		const interval = setInterval(() => {
			setStep((s) => (s + 1) % 4);
		}, 1500);
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
						Zero-Latency Profile Access
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-zinc-400 max-w-2xl text-lg leading-relaxed"
					>
						Traditional architectures require a database round-trip for every request to fetch user sessions. Rivet Actors keep the user's state hot in memory at the edge.
					</motion.p>
				</div>

				<div className="grid lg:grid-cols-2 gap-12 items-center">
					{/* Visualization */}
					<div className="relative h-80 rounded-2xl border border-white/10 bg-zinc-900/20 flex flex-col items-center justify-center overflow-hidden p-8">
						<div className="relative w-full h-full flex items-center justify-between z-10 px-8">
							{/* User */}
							<div className="flex flex-col items-center gap-2 z-10">
								<div className={`w-12 h-12 rounded-full bg-white flex items-center justify-center shadow-[0_0_20px_white] ${step === 0 ? "scale-110" : "scale-100"} transition-transform`}>
									<Users className="w-6 h-6 text-black" />
								</div>
								<span className="text-xs font-mono text-zinc-400">Request</span>
							</div>

							{/* Path */}
							<div className="flex-1 h-[2px] bg-zinc-800 relative mx-4 overflow-hidden">
								{/* Packet */}
								<div className={`absolute top-0 left-0 h-full w-12 bg-cyan-400 blur-[2px] transition-all duration-[1000ms] linear ${step === 1 ? "translate-x-full opacity-100" : "translate-x-0 opacity-0"}`} />
								<div className={`absolute top-0 right-0 h-full w-12 bg-cyan-400 blur-[2px] transition-all duration-[1000ms] linear ${step === 3 ? "-translate-x-full opacity-100" : "translate-x-0 opacity-0"}`} style={{ transform: step === 3 ? "translateX(-500%)" : "translateX(0)" }} />
							</div>

							{/* The Session Actor */}
							<div className="flex flex-col items-center gap-2 z-10">
								<div className={`w-16 h-16 rounded-xl border-2 ${step === 2 ? "border-cyan-400 bg-cyan-500/20 shadow-[0_0_30px_rgba(34,211,238,0.3)]" : "border-zinc-700 bg-zinc-900"} flex items-center justify-center transition-all duration-300`}>
									<Fingerprint className={`w-8 h-8 ${step === 2 ? "text-cyan-400" : "text-zinc-600"}`} />
								</div>
								<span className="text-xs font-mono text-zinc-400">Session Actor</span>
								{step === 2 && (
									<div className="absolute -top-8 bg-zinc-800 text-cyan-400 text-[10px] px-2 py-1 rounded border border-cyan-500/30">
										Memory Hit
									</div>
								)}
							</div>
						</div>

						{/* Comparison text */}
						<div className="absolute bottom-6 left-0 right-0 flex justify-center gap-8 text-[10px] font-mono uppercase tracking-widest">
							<div className="flex items-center gap-2 text-zinc-500 opacity-50">
								<div className="w-2 h-2 rounded-full bg-zinc-600" />
								Postgres: 45ms
							</div>
							<div className="flex items-center gap-2 text-cyan-400">
								<div className="w-2 h-2 rounded-full bg-cyan-400 animate-pulse" />
								Rivet: 2ms
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
								<Zap className="w-5 h-5 text-cyan-400" />
								Instant Read/Write
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								State is stored in RAM. Reading user preferences or checking permissions happens in microseconds, not milliseconds.
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
								<Globe className="w-5 h-5 text-blue-400" />
								Edge Routing
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								Actors are automatically instantiated in the region closest to the user, ensuring global low-latency access.
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
								<Lock className="w-5 h-5 text-purple-400" />
								Secure by Default
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								Isolated memory spaces for every user. No risk of cross-tenant data leaks or cache poisoning.
							</p>
						</motion.div>
					</div>
				</div>
			</div>
		</section>
	);
};

const SessionFeatures = () => {
	const features = [
		{
			title: "Live Sync",
			description: "Changes to session data (e.g. dark mode toggle) are broadcast instantly to all open tabs or devices via WebSockets.",
			icon: Wifi,
			color: "cyan",
		},
		{
			title: "TTL & Expiry",
			description: "Automatically destroy session actors after a period of inactivity to free up resources and enforce security.",
			icon: Clock,
			color: "orange",
		},
		{
			title: "Cart Persistence",
			description: "Never lose a shopping cart item again. State survives server restarts and follows the user across devices.",
			icon: ShoppingCart,
			color: "emerald",
		},
		{
			title: "Presence",
			description: "Know exactly when a user is online. The Actor exists only while the user is connected (or for a grace period).",
			icon: Activity,
			color: "blue",
		},
		{
			title: "Auth Tokens",
			description: "Generate and validate JWTs directly within the Actor. Revoke tokens instantly by killing the Actor.",
			icon: Shield,
			color: "red",
		},
		{
			title: "Device Handover",
			description: "Start a task on mobile, finish on desktop. The shared Actor state creates a seamless continuity.",
			icon: Smartphone,
			color: "purple",
		},
	];

	return (
		<section className="py-32 bg-zinc-900/20 relative">
			<div className="max-w-7xl mx-auto px-6">
				<div className="mb-20 text-center">
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						More than just a Cookie
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-zinc-400"
					>
						Turn your passive session store into an active engine for user experience.
					</motion.p>
				</div>

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

const UseCases = () => (
	<section className="py-24 bg-black border-t border-white/5">
		<div className="max-w-7xl mx-auto px-6">
			<div className="grid md:grid-cols-2 gap-16 items-center">
				<div>
					<Badge text="Case Study" color="cyan" />
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						Real-time E-Commerce
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg text-zinc-400 mb-8 leading-relaxed"
					>
						A high-volume storefront where cart inventory is reserved instantly and stock levels update live.
					</motion.p>
					<motion.ul
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="space-y-4"
					>
						{[
							"Inventory Locks: Items reserved in Actor memory",
							"Dynamic Pricing: Personalized discounts applied instantly",
							"Cross-Device: Cart updates on phone appear on laptop",
						].map((item, i) => (
							<li key={i} className="flex items-center gap-3 text-zinc-300">
								<div className="w-5 h-5 rounded-full bg-cyan-500/20 flex items-center justify-center">
									<Check className="w-3 h-3 text-cyan-400" />
								</div>
								{item}
							</li>
						))}
					</motion.ul>
				</div>
				<motion.div
					initial={{ opacity: 0, x: 20 }}
					whileInView={{ opacity: 1, x: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="relative"
				>
					<div className="absolute inset-0 bg-gradient-to-r from-cyan-500/20 to-transparent rounded-2xl blur-2xl" />
					<div className="relative rounded-2xl border border-white/10 bg-zinc-900 p-6 shadow-2xl">
						<div className="flex items-center justify-between mb-6 border-b border-white/5 pb-4">
							<div className="flex items-center gap-3">
								<div className="w-8 h-8 rounded bg-cyan-500/20 flex items-center justify-center">
									<ShoppingCart className="w-5 h-5 text-cyan-400" />
								</div>
								<div>
									<div className="text-sm font-medium text-white">User: alex_92</div>
									<div className="text-xs text-zinc-500">Session Active (2 devices)</div>
								</div>
							</div>
							<div className="px-2 py-1 rounded bg-cyan-500/10 text-cyan-400 text-xs border border-cyan-500/20">Online</div>
						</div>
						<div className="space-y-4 text-sm font-mono">
							<div className="p-3 rounded bg-zinc-950 border border-white/5 text-zinc-400 flex justify-between">
								<span>Item Added: Mechanical Keyboard</span>
								<span className="text-green-400">+$149.00</span>
							</div>
							<div className="p-3 rounded bg-cyan-900/20 border border-cyan-500/30 text-cyan-200">
								Broadcast: "cart_update" -&gt; 2 clients
							</div>
							<div className="flex gap-2">
								<div className="h-1.5 w-full bg-zinc-800 rounded-full overflow-hidden">
									<div className="h-full bg-cyan-500 w-3/4 animate-[pulse_3s_infinite]" />
								</div>
							</div>
						</div>
					</div>
				</motion.div>
			</div>
		</div>
	</section>
);

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
				Works with your auth provider
			</motion.h2>
			<div className="flex flex-wrap justify-center gap-4">
				{["Clerk", "Auth0", "Supabase Auth", "NextAuth.js", "Stytch", "Firebase"].map((tech, i) => (
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

export default function UserSessionStorePage() {
	return (
		<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-cyan-500/30 selection:text-cyan-200">
			<main>
				<Hero />
				<LatencyArchitecture />
				<SessionFeatures />
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
							Stop querying the database.
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-lg text-zinc-400 mb-10 leading-relaxed"
						>
							Move your user state to the edge with Rivet Actors.
						</motion.p>
						<motion.div
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.2 }}
							className="flex flex-col sm:flex-row items-center justify-center gap-4"
						>
							<button className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black shadow-sm hover:bg-zinc-200 transition-colors">
								Start Building
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

