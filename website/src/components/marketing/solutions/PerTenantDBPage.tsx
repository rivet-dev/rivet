"use client";

import {
	Zap,
	ArrowRight,
	Database,
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
const Badge = ({ text }: { text: string }) => (
	<div className="inline-flex items-center gap-2 rounded-full border border-white/10 px-3 py-1 text-xs text-zinc-400 mb-6">
		<span className="h-1.5 w-1.5 rounded-full bg-[#FF4500]" />
		{text}
	</div>
);

const CodeBlock = ({ code, fileName = "tenant.ts" }: { code: string; fileName?: string }) => {
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
			else if (["state", "actions", "broadcast", "c", "tenant", "data", "query", "insert", "update", "delete", "get", "set"].includes(trimmed)) {
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
					<Badge text="Per-Tenant Databases" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="text-4xl md:text-6xl font-normal text-white tracking-tight leading-[1.1] mb-6"
					>
						Persistent State for Every Customer.
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-base text-zinc-500 leading-relaxed mb-8 max-w-lg"
					>
						Don't leak data between rows. Give every tenant their own isolated Actor with private in-memory state. Zero latency, instant provisioning, and total data sovereignty.
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
	</section>
);

const IsolationArchitecture = () => {
	return (
		<section className="border-t border-white/10 py-48">
			<div className="max-w-7xl mx-auto px-6">
				<div className="mb-16 text-center">
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-2"
					>
						The Silo Model
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-base leading-relaxed text-zinc-500 max-w-2xl mx-auto"
					>
						In a traditional SaaS, one bad query from Tenant A can slow down Tenant B. With Rivet, every tenant lives in their own process with their own resources.
					</motion.p>
				</div>

				{/* Diagram Container */}
				<div className="flex flex-col items-center justify-center w-full max-w-4xl mx-auto">

					{/* The Router */}
					<div className="w-full max-w-[280px] p-6 rounded-lg bg-black border border-white/10 flex flex-col items-center text-center z-20">
						<Network className="w-8 h-8 text-[#FF4500] mb-4" />
						<h3 className="text-lg font-medium text-white mb-1">Rivet Router</h3>
						<p className="text-xs text-zinc-500">Routes to Tenant Actors</p>
					</div>

					{/* Vertical line from router */}
					<div className="h-8 w-[2px] bg-zinc-800 relative overflow-hidden">
						<div className="absolute inset-0 bg-gradient-to-b from-transparent via-zinc-500 to-transparent h-1/2 animate-[flow-v_2s_linear_infinite]" />
					</div>

					{/* Horizontal spread bar */}
					<div className="w-full h-[2px] bg-zinc-800" />

					{/* Three vertical lines + Tenant cards */}
					<div className="w-full grid grid-cols-3 gap-8">
						{/* Tenant A column */}
						<div className="flex flex-col items-center">
							<div className="h-8 w-[2px] bg-zinc-800 relative overflow-hidden">
								<div className="absolute inset-0 bg-gradient-to-b from-transparent via-zinc-500 to-transparent h-1/2 animate-[flow-v_2s_linear_infinite_0.2s]" />
							</div>
							<div className="p-6 rounded-lg bg-black border border-white/10 flex flex-col items-center text-center relative hover:border-white/20 transition-colors w-full">
								<div className="absolute -top-3 px-3 py-0.5 bg-zinc-900 border border-white/10 rounded-full text-[10px] text-zinc-400 font-mono">Tenant A</div>
								<Database className="w-6 h-6 text-zinc-400 mb-4" />
								<div className="w-full h-1 bg-zinc-800 rounded-full overflow-hidden mb-2">
									<div className="h-full w-[40%] bg-zinc-500" />
								</div>
								<span className="text-[10px] text-zinc-500">12MB • Active</span>
							</div>
						</div>

						{/* Tenant B column */}
						<div className="flex flex-col items-center">
							<div className="h-8 w-[2px] bg-zinc-800 relative overflow-hidden">
								<div className="absolute inset-0 bg-gradient-to-b from-transparent via-zinc-500 to-transparent h-1/2 animate-[flow-v_2s_linear_infinite_0.4s]" />
							</div>
							<div className="p-6 rounded-lg bg-black border border-white/10 flex flex-col items-center text-center relative hover:border-white/20 transition-colors w-full">
								<div className="absolute -top-3 px-3 py-0.5 bg-zinc-900 border border-white/10 rounded-full text-[10px] text-zinc-400 font-mono">Tenant B</div>
								<Database className="w-6 h-6 text-zinc-400 mb-4" />
								<div className="w-full h-1 bg-zinc-800 rounded-full overflow-hidden mb-2">
									<div className="h-full w-[80%] bg-zinc-500" />
								</div>
								<span className="text-[10px] text-zinc-500">1.4GB • Active</span>
							</div>
						</div>

						{/* Tenant C column */}
						<div className="flex flex-col items-center">
							<div className="h-8 w-[2px] bg-zinc-800 relative overflow-hidden">
								<div className="absolute inset-0 bg-gradient-to-b from-transparent via-zinc-500 to-transparent h-1/2 animate-[flow-v_2s_linear_infinite_0.6s]" />
							</div>
							<div className="p-6 rounded-lg bg-black border border-white/10 flex flex-col items-center text-center relative hover:border-white/20 transition-colors w-full">
								<div className="absolute -top-3 px-3 py-0.5 bg-zinc-900 border border-white/10 rounded-full text-[10px] text-zinc-400 font-mono">Tenant C</div>
								<Moon className="w-6 h-6 text-zinc-500 mb-4 opacity-50" />
								<div className="w-full h-1 bg-zinc-800 rounded-full overflow-hidden mb-2">
									<div className="h-full w-0 bg-zinc-600" />
								</div>
								<span className="text-[10px] text-zinc-500">0MB • Sleeping</span>
							</div>
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
		},
		{
			title: "Instant Provisioning",
			description: "Create a new tenant store in milliseconds. Just spawn an actor; no Terraform required.",
			icon: Rocket,
		},
		{
			title: "Schema Isolation",
			description: "Every tenant can have a different state shape. Roll out data migrations gradually, tenant by tenant.",
			icon: FileJson,
		},
		{
			title: "Connection Limits",
			description: "Stop worrying about Postgres connection limits. Each actor has exclusive access to its own isolated state.",
			icon: Gauge,
		},
		{
			title: "Data Sovereignty",
			description: "Easily export a single tenant's state as a JSON file. Perfect for GDPR takeouts or backups.",
			icon: Shield,
		},
		{
			title: "Cost Efficiency",
			description: "Sleeping tenants cost nothing. You only pay for active CPU/RAM when the state is being accessed.",
			icon: Coins,
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
					<h2 className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-2">State Superpowers</h2>
					<p className="text-base leading-relaxed text-zinc-500">The benefits of embedded state with the scale of the cloud.</p>
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
						B2B CRM Platform
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-base text-zinc-500 mb-8 leading-relaxed"
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
									<Table2 className="w-5 h-5 text-white" />
								</div>
								<div>
									<div className="text-sm font-medium text-white">Tenant: Acme Corp</div>
									<div className="text-xs text-zinc-500">DB Size: 450MB</div>
								</div>
							</div>
							<div className="px-2 py-1 rounded bg-green-500/10 text-green-400 text-xs border border-green-500/20">Online</div>
						</div>
						<div className="space-y-4 text-sm font-mono">
							<div className="p-3 rounded bg-zinc-950 border border-white/10 text-zinc-400">&gt; GET /leads?status=new</div>
							<div className="p-3 rounded bg-zinc-950 border border-white/10 text-white">&lt; Result: 14,203 objects (Returned in 4ms)</div>
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
		<section className="border-t border-white/10 py-48">
			<div className="max-w-7xl mx-auto px-6">
				<motion.h2
					initial={{ opacity: 0, y: 20 }}
					whileInView={{ opacity: 1, y: 0 }}
					viewport={{ once: true }}
					transition={{ duration: 0.5 }}
					className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-2 text-center"
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
								<Key className="h-4 w-4" />
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
				className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-2"
			>
				Works with your stack
			</motion.h2>
			<div className="flex flex-wrap justify-center gap-4 mt-12">
				{["Drizzle", "Kysely", "Zod", "Prisma (JSON)", "TypeORM"].map((tech, i) => (
					<motion.div
						key={tech}
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: i * 0.05 }}
						className="rounded-md border border-white/5 px-2 py-1 text-xs bg-black/50 text-zinc-400 font-mono hover:text-white hover:border-white/30 transition-colors cursor-default backdrop-blur-sm"
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
		<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-[#FF4500]/30 selection:text-orange-200">
			<main>
				<Hero />
				<IsolationArchitecture />
				<StateFeatures />
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
							Isolate your data.
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-base text-zinc-500 mb-10 leading-relaxed"
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
							<a href="/docs" className="inline-flex items-center justify-center whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200">
								Start Building Now
							</a>
							<a href="/docs/actors/persistence" className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20">
								Read the Docs
							</a>
						</motion.div>
					</div>
				</section>
			</main>
		</div>
	);
}
