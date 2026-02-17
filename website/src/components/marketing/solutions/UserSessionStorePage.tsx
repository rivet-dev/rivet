"use client";

import {
	Zap,
	Globe,
	ArrowRight,
	Database,
	Check,
	RefreshCw,
	Lock,
	ShoppingCart,
	ShoppingBag,
	Search,
	Lightbulb,
	Users,
	CreditCard,
	Smartphone,
} from "lucide-react";
import { motion } from "framer-motion";

// --- Shared Design Components ---
const Badge = ({ text }: { text: string }) => (
	<div className="inline-flex items-center gap-2 rounded-full border border-white/10 px-3 py-1 text-xs text-zinc-400 mb-6">
		<span className="h-1.5 w-1.5 rounded-full bg-[#FF4500]" />
		{text}
	</div>
);

const CodeBlock = ({ code, fileName = "cart.ts" }: { code: string; fileName?: string }) => {
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
			else if (["state", "actions", "broadcast", "c", "cart", "items", "total", "addItem", "removeItem", "checkout", "push", "filter"].includes(trimmed)) {
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
					<Badge text="Real-time Commerce" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="text-4xl md:text-6xl font-normal text-white tracking-tight leading-[1.1] mb-6"
					>
						The Cart that Never Forgets
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-base text-zinc-500 leading-relaxed mb-8 max-w-lg"
					>
						Eliminate database contention on launch day. Use Rivet Actors to hold shopping carts, reserve inventory, and power agentic search sessions across devices.
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
	</section>
);

const CommerceArchitecture = () => {
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
						The Universal Session
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-base leading-relaxed text-zinc-500 max-w-2xl"
					>
						Stop syncing database rows to keep devices in sync. With Rivet, the Actor <em>is</em> the session. All devices connect to the same living process.
					</motion.p>
				</div>

				<div className="relative h-[450px] w-full rounded-lg border border-white/10 bg-black flex items-center justify-center overflow-hidden p-8">
					<div className="absolute inset-0 bg-[linear-gradient(rgba(255,255,255,0.03)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.03)_1px,transparent_1px)] bg-[size:40px_40px]" />

					<div className="relative z-10 w-full max-w-4xl grid grid-cols-3 gap-8 items-center">
						{/* Left: Mobile User */}
						<div className="flex flex-col items-center gap-4 group">
							<div className="w-20 h-32 rounded-2xl bg-zinc-950 border border-white/10 flex flex-col items-center justify-center shadow-lg relative group-hover:border-white/20 transition-colors">
								<div className="absolute top-2 w-8 h-1 bg-zinc-800 rounded-full" />
								<Smartphone className="w-8 h-8 text-white mb-2" />
								<div className="text-[10px] text-zinc-500 font-mono">Mobile App</div>

								<div className="absolute -right-12 top-8 bg-[#FF4500]/10 border border-[#FF4500]/20 text-[#FF4500] text-[10px] px-2 py-1 rounded backdrop-blur-md">
									+1 Sneakers
								</div>
							</div>
						</div>

						{/* Middle: The Session Actor */}
						<div className="flex flex-col items-center justify-center relative">
							<div className="w-32 h-32 rounded-full border-2 border-white/20 bg-black flex flex-col items-center justify-center relative">
								<div className="absolute inset-0 rounded-full border border-white/10 animate-[spin_10s_linear_infinite]" />
								<ShoppingCart className="w-10 h-10 text-white mb-2" />
								<div className="text-xs font-mono text-zinc-300 font-medium">Cart Actor</div>
								<div className="text-[10px] text-zinc-500 font-mono">1 Item • $120</div>
							</div>
						</div>

						{/* Right: Desktop User */}
						<div className="flex flex-col items-center gap-4 group">
							<div className="w-32 h-24 rounded-lg bg-zinc-950 border border-white/10 flex flex-col items-center justify-center shadow-lg relative group-hover:border-white/20 transition-colors">
								<div className="absolute bottom-[-20px] w-24 h-2 bg-zinc-800 rounded-b-lg" />
								<div className="absolute bottom-[-20px] w-4 h-4 bg-zinc-800 skew-x-12" />
								<Globe className="w-8 h-8 text-white mb-2" />
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
		},
		{
			title: "Universal Cart",
			description: "The same cart state follows the user from mobile to desktop to tablet. No refresh required.",
			icon: ShoppingBag,
		},
		{
			title: "Agentic Search",
			description: "Use the actor's session history to feed a vector search instantly, re-ranking results based on user intent.",
			icon: Search,
		},
		{
			title: "Live Recommendations",
			description: "Update suggested products in real-time as the user scrolls, based on their immediate view history held in RAM.",
			icon: Lightbulb,
		},
		{
			title: "Flash Sales",
			description: "Handle huge spikes in traffic. 100k users can add to cart simultaneously without locking your database.",
			icon: Zap,
		},
		{
			title: "Collaborative Shopping",
			description: "Allow multiple users to view and edit the same cart (e.g. families or procurement teams).",
			icon: Users,
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
					<h2 className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-2">Built for Conversion</h2>
					<p className="text-base leading-relaxed text-zinc-500">Speed is revenue. Rivet makes your store feel instant.</p>
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
						High-Volume Drop
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-base text-zinc-500 mb-8 leading-relaxed"
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
								className="flex items-center gap-3 text-zinc-300 text-sm"
							>
								<Check className="w-4 h-4 text-[#FF4500]" />
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
					<div className="relative rounded-lg border border-white/10 bg-black p-6">
						<div className="flex items-center justify-between mb-6 border-b border-white/10 pb-4">
							<div className="flex items-center gap-3">
								<div className="w-8 h-8 rounded bg-white/5 flex items-center justify-center border border-white/10">
									<ShoppingBag className="w-5 h-5 text-white" />
								</div>
								<div>
									<div className="text-sm font-medium text-white">Item: Retro High '85</div>
									<div className="text-xs text-zinc-500">Stock: 42 / 5000</div>
								</div>
							</div>
							<div className="px-2 py-1 rounded bg-[#FF4500]/10 text-[#FF4500] text-xs border border-[#FF4500]/20">Selling Fast</div>
						</div>
						<div className="space-y-4 text-sm font-mono">
							<div className="p-3 rounded bg-zinc-950 border border-white/10 text-zinc-400 flex justify-between">
								<span>User: guest_9921</span>
								<span className="text-green-400">Checkout Success</span>
							</div>
							<div className="p-3 rounded bg-zinc-950 border border-white/10 text-zinc-400 flex justify-between">
								<span>User: guest_4402</span>
								<span className="text-red-400">Cart Expired</span>
							</div>
							<div className="w-full bg-zinc-800 rounded-full h-1.5 overflow-hidden">
								<div className="bg-[#FF4500] h-full w-[98%] animate-pulse" />
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
		<section className="border-t border-white/10 py-48">
			<div className="max-w-7xl mx-auto px-6">
				<motion.h2
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-2"
				>
					Commerce at Scale
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
								<CreditCard className="h-4 w-4" />
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
				{["Shopify", "Stripe", "Medusa", "BigCommerce", "Adyen", "Algolia"].map((tech, i) => (
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

export default function CommercePage() {
	return (
		<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-[#FF4500]/30 selection:text-orange-200">
			<main>
				<Hero />
				<CommerceArchitecture />
				<CommerceFeatures />
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
							className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-2"
						>
							Sell without limits
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-base text-zinc-500 mb-10 leading-relaxed"
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
							<a href="/docs" className="inline-flex items-center justify-center whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200">
								Start Building Now
							</a>
							<a href="https://github.com/rivet-dev/rivet/tree/main/examples/state" className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white">
								View Example
							</a>
						</motion.div>
					</div>
				</section>
			</main>
		</div>
	);
}
