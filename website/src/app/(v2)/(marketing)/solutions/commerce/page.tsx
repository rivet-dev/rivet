"use client";

import { useState } from "react";
import {
	Terminal,
	Zap,
	Globe,
	ArrowRight,
	Database,
	Check,
	RefreshCw,
	Clock,
	Lock,
	ShoppingCart,
	ShoppingBag,
	Search,
	Lightbulb,
	Users,
	CreditCard,
	Smartphone,
	Link as LinkIcon,
} from "lucide-react";
import { motion } from "framer-motion";

// --- Shared Design Components ---
const Badge = ({ text, color = "cyan" }) => {
	const colorClasses = {
		orange: "text-orange-400 border-orange-500/20 bg-orange-500/10",
		blue: "text-blue-400 border-blue-500/20 bg-blue-500/10",
		red: "text-red-400 border-red-500/20 bg-red-500/10",
		pink: "text-pink-400 border-pink-500/20 bg-pink-500/10",
		indigo: "text-indigo-400 border-indigo-500/20 bg-indigo-500/10",
		cyan: "text-cyan-400 border-cyan-500/20 bg-cyan-500/10",
	};

	return (
		<div
			className={`inline-flex items-center gap-2 px-3 py-1 rounded-full border text-xs font-medium mb-8 transition-colors cursor-default ${colorClasses[color]}`}
		>
			<span className={`w-1.5 h-1.5 rounded-full ${color === "orange" ? "bg-orange-400" : color === "blue" ? "bg-blue-400" : color === "red" ? "bg-red-400" : color === "pink" ? "bg-pink-400" : color === "indigo" ? "bg-indigo-400" : "bg-cyan-400"} animate-pulse`} />
			{text}
		</div>
	);
};

const CodeBlock = ({ code, fileName = "cart.ts" }) => {
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

											if (["import", "from", "export", "const", "return", "async", "await", "function", "if", "throw"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-purple-400">{part}</span>);
											} else if (["actor", "recommend", "addItem", "rpc", "inventory", "reserve", "ai", "broadcast"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-400">{part}</span>);
											} else if (["state", "actions", "items", "history", "productId", "qty", "stock", "recent", "slice"].includes(trimmed)) {
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
			case "red":
				return {
					bg: "bg-red-500/10",
					text: "text-red-400",
					hoverBg: "group-hover:bg-red-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(239,68,68,0.5)]",
					border: "border-red-500",
					glow: "rgba(239,68,68,0.15)",
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
			case "indigo":
				return {
					bg: "bg-indigo-500/10",
					text: "text-indigo-400",
					hoverBg: "group-hover:bg-indigo-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(99,102,241,0.5)]",
					border: "border-indigo-500",
					glow: "rgba(99,102,241,0.15)",
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
		<div className="absolute top-0 left-1/2 -translate-x-1/2 w-[1000px] h-[500px] bg-cyan-500/[0.03] blur-[100px] rounded-full pointer-events-none" />

		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<Badge text="Real-time Commerce" color="cyan" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="text-5xl md:text-7xl font-medium text-white tracking-tight leading-[1.1] mb-6"
					>
						The Cart that <br />
						<span className="text-cyan-400">Never Forgets.</span>
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg md:text-xl text-zinc-400 leading-relaxed mb-8 max-w-lg"
					>
						Eliminate database contention on launch day. Use Rivet Actors to hold shopping carts, reserve inventory, and power agentic search sessions across devices.
					</motion.p>

					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="flex flex-col sm:flex-row items-center gap-4"
					>
						<button className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black shadow-sm hover:bg-zinc-200 transition-colors gap-2">
							Start Building
							<ArrowRight className="w-4 h-4" />
						</button>
					</motion.div>
				</div>

				<div className="flex-1 w-full max-w-xl">
					<div className="relative">
						<div className="absolute -inset-1 bg-gradient-to-r from-cyan-500/20 to-blue-500/20 rounded-xl blur opacity-40" />
						<CodeBlock
							fileName="shopping_cart.ts"
							code={`import { actor } from "rivetkit";

export const shoppingCart = actor({
  state: { items: [], history: [] },

  actions: {
    // Instant recommendations based on hot state
    recommend: async (c) => {
      const recent = c.state.history.slice(-5);
      return await c.ai.recommend(recent);
    },

    addItem: async (c, { productId, qty }) => {
      const stock = await c.rpc.inventory.reserve(productId, qty);
      if (!stock) throw "Out of Stock";

      c.state.items.push({ productId, qty });
      c.broadcast("cart_updated", c.state);
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

const CommerceArchitecture = () => {
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
						The Universal Session
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-zinc-400 max-w-2xl mx-auto text-lg leading-relaxed"
					>
						Stop syncing database rows to keep devices in sync. With Rivet, the Actor <em>is</em> the session. All devices connect to the same living process.
					</motion.p>
				</div>

				<div className="relative h-[450px] w-full rounded-2xl border border-white/10 bg-zinc-900/20 flex items-center justify-center overflow-hidden p-8">
					<div className="absolute inset-0 bg-[linear-gradient(rgba(255,255,255,0.03)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.03)_1px,transparent_1px)] bg-[size:40px_40px]" />

					<div className="relative z-10 w-full max-w-4xl grid grid-cols-3 gap-8 items-center">
						{/* Left: Mobile User */}
						<div className="flex flex-col items-center gap-4 group">
							<div className="w-20 h-32 rounded-2xl bg-zinc-950 border border-zinc-700 flex flex-col items-center justify-center shadow-lg relative group-hover:border-cyan-500/50 transition-colors">
								<div className="absolute top-2 w-8 h-1 bg-zinc-800 rounded-full" />
								<Smartphone className="w-8 h-8 text-cyan-400 mb-2" />
								<div className="text-[10px] text-zinc-500 font-mono">Mobile App</div>

								<div className="absolute -right-12 top-8 bg-cyan-500/10 border border-cyan-500/20 text-cyan-300 text-[10px] px-2 py-1 rounded backdrop-blur-md">
									+1 Sneakers
								</div>
							</div>
						</div>

						{/* Middle: The Session Actor */}
						<div className="flex flex-col items-center justify-center relative">
							<div className="w-32 h-32 rounded-full border-2 border-cyan-500/30 bg-black flex flex-col items-center justify-center relative shadow-[0_0_60px_rgba(34,211,238,0.2)]">
								<div className="absolute inset-0 rounded-full border border-cyan-400/20 animate-[spin_10s_linear_infinite]" />
								<ShoppingCart className="w-10 h-10 text-cyan-400 mb-2" />
								<div className="text-xs font-mono text-zinc-300 font-medium">Cart Actor</div>
								<div className="text-[10px] text-zinc-500 font-mono">1 Item • $120</div>
							</div>
						</div>

						{/* Right: Desktop User */}
						<div className="flex flex-col items-center gap-4 group">
							<div className="w-32 h-24 rounded-lg bg-zinc-950 border border-zinc-700 flex flex-col items-center justify-center shadow-lg relative group-hover:border-cyan-500/50 transition-colors">
								<div className="absolute bottom-[-20px] w-24 h-2 bg-zinc-800 rounded-b-lg" />
								<div className="absolute bottom-[-20px] w-4 h-4 bg-zinc-800 skew-x-12" />
								<Globe className="w-8 h-8 text-cyan-400 mb-2" />
								<div className="text-[10px] text-zinc-500 font-mono">Web Store</div>

								<div className="absolute -top-3 -right-3 bg-green-500/20 border border-green-500/50 text-green-400 text-[10px] px-2 py-0.5 rounded-full flex items-center gap-1">
									<RefreshCw className="w-3 h-3" /> Synced
								</div>
							</div>
						</div>
					</div>
				</div>
			</div>
		</section>
	);
};

const CommerceFeatures = () => {
	const features = [
		{
			title: "Inventory Reservation",
			description: "Hold items in the actor's memory for 10 minutes. If the user doesn't checkout, release stock instantly.",
			icon: Lock,
			color: "cyan",
		},
		{
			title: "Universal Cart",
			description: "The same cart state follows the user from mobile to desktop to tablet. No refresh required.",
			icon: ShoppingBag,
			color: "blue",
		},
		{
			title: "Agentic Search",
			description: "Use the actor's session history to feed a vector search instantly, re-ranking results based on user intent.",
			icon: Search,
			color: "indigo",
		},
		{
			title: "Live Recommendations",
			description: "Update suggested products in real-time as the user scrolls, based on their immediate view history held in RAM.",
			icon: Lightbulb,
			color: "orange",
		},
		{
			title: "Flash Sales",
			description: "Handle huge spikes in traffic. 100k users can add to cart simultaneously without locking your database.",
			icon: Zap,
			color: "red",
		},
		{
			title: "Collaborative Shopping",
			description: "Allow multiple users to view and edit the same cart (e.g. families or procurement teams).",
			icon: Users,
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
					<h2 className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight">Built for Conversion</h2>
					<p className="text-zinc-400 text-lg leading-relaxed">Speed is revenue. Rivet makes your store feel instant.</p>
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
					<Badge text="Case Study" color="cyan" />
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						High-Volume Drop
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg text-zinc-400 mb-8 leading-relaxed"
					>
						A streetwear brand launching a limited edition collection. 50,000 users arrive in 60 seconds.
					</motion.p>
					<ul className="space-y-4">
						{["No Overselling: Atomic decrement of inventory in memory", "Queueing: Fair FIFO processing of checkout requests", "Instant Feedback: UI updates immediately on success/fail"].map((item, i) => (
							<motion.li
								key={i}
								initial={{ opacity: 0, x: -20 }}
								whileInView={{ opacity: 1, x: 0 }}
								viewport={{ once: true }}
								transition={{ duration: 0.5, delay: 0.2 + i * 0.1 }}
								className="flex items-center gap-3 text-zinc-300"
							>
								<div className="w-5 h-5 rounded-full bg-cyan-500/20 flex items-center justify-center">
									<Check className="w-3 h-3 text-cyan-400" />
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
					<div className="absolute inset-0 bg-gradient-to-r from-cyan-500/20 to-transparent rounded-2xl blur-2xl" />
					<div className="relative rounded-2xl border border-white/10 bg-zinc-900 p-6 shadow-2xl">
						<div className="flex items-center justify-between mb-6 border-b border-white/5 pb-4">
							<div className="flex items-center gap-3">
								<div className="w-8 h-8 rounded bg-cyan-500/20 flex items-center justify-center">
									<ShoppingBag className="w-5 h-5 text-cyan-400" />
								</div>
								<div>
									<div className="text-sm font-medium text-white">Item: Retro High '85</div>
									<div className="text-xs text-zinc-500">Stock: 42 / 5000</div>
								</div>
							</div>
							<div className="px-2 py-1 rounded bg-red-500/10 text-red-400 text-xs border border-red-500/20">Selling Fast</div>
						</div>
						<div className="space-y-4 text-sm font-mono">
							<div className="p-3 rounded bg-zinc-950 border border-white/5 text-zinc-400 flex justify-between">
								<span>User: guest_9921</span>
								<span className="text-green-400">Checkout Success</span>
							</div>
							<div className="p-3 rounded bg-zinc-950 border border-white/5 text-zinc-400 flex justify-between">
								<span>User: guest_4402</span>
								<span className="text-red-400">Cart Expired</span>
							</div>
							<div className="w-full bg-zinc-800 rounded-full h-1.5 overflow-hidden">
								<div className="bg-cyan-500 h-full w-[98%] animate-pulse" />
							</div>
							<div className="text-right text-[10px] text-zinc-500">Inventory Load: 98%</div>
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
			title: "Marketplaces",
			desc: "Coordinate buyers and sellers in real-time auctions with instant bid updates.",
		},
		{
			title: "Ticketing",
			desc: "Reserve seats for concerts or events. Prevent double-booking with atomic locks.",
		},
		{
			title: "Food Delivery",
			desc: "Track order status, driver location, and inventory in a single stateful entity.",
		},
		{
			title: "Subscription Apps",
			desc: "Manage access rights and feature gating dynamically based on payment status.",
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
					Commerce at Scale
				</motion.h2>
				<div className="grid md:grid-cols-2 lg:grid-cols-4 gap-6">
					{cases.map((c, i) => (
						<motion.div
							key={i}
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: i * 0.05 }}
							className="p-6 rounded-xl border border-white/10 bg-zinc-900/30 hover:bg-cyan-900/10 hover:border-cyan-500/30 transition-colors group"
						>
							<div className="mb-4">
								<CreditCard className="w-6 h-6 text-cyan-500 group-hover:scale-110 transition-transform" />
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
				{["Shopify", "Stripe", "Medusa", "BigCommerce", "Adyen", "Algolia"].map((tech, i) => (
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

export default function CommercePage() {
	return (
		<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-cyan-500/30 selection:text-cyan-200">
			<main>
				<Hero />
				<CommerceArchitecture />
				<CommerceFeatures />
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
							Sell without limits.
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-lg text-zinc-400 mb-10 leading-relaxed"
						>
							Build shopping experiences that are fast, reliable, and instantly synchronized.
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
								View Examples
							</button>
						</motion.div>
					</div>
				</section>
			</main>
		</div>
	);
}

