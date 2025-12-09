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
	Play,
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
} from "lucide-react";
import { motion } from "framer-motion";

// --- Shared Design Components ---
const Badge = ({ text, color = "red" }) => {
	const colorClasses = {
		orange: "text-orange-400 border-orange-500/20 bg-orange-500/10",
		blue: "text-blue-400 border-blue-500/20 bg-blue-500/10",
		purple: "text-purple-400 border-purple-500/20 bg-purple-500/10",
		emerald: "text-emerald-400 border-emerald-500/20 bg-emerald-500/10",
		red: "text-red-400 border-red-500/20 bg-red-500/10",
	};

	return (
		<div
			className={`inline-flex items-center gap-2 px-3 py-1 rounded-full border text-xs font-medium mb-8 transition-colors cursor-default ${colorClasses[color]}`}
		>
			<span
				className={`w-1.5 h-1.5 rounded-full ${
					color === "orange"
						? "bg-orange-400"
						: color === "blue"
							? "bg-blue-400"
							: color === "purple"
								? "bg-purple-400"
								: color === "emerald"
									? "bg-emerald-400"
									: "bg-red-400"
				} animate-pulse`}
			/>
			{text}
		</div>
	);
};

const CodeBlock = ({ code, fileName = "match.ts" }) => {
	return (
		<div className="relative group rounded-xl overflow-hidden border border-white/10 bg-background shadow-2xl">
			<div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-white/30 to-transparent z-10" />
			<div className="flex items-center justify-between px-4 py-3 border-b border-white/5 bg-white/5">
				<div className="flex items-center gap-2">
					<div className="w-3 h-3 rounded-full bg-red-500/20 border border-red-500/50" />
					<div className="w-3 h-3 rounded-full bg-yellow-500/20 border border-yellow-500/50" />
					<div className="w-3 h-3 rounded-full bg-green-500/20 border border-green-500/50" />
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
										const parts = current
											.split(/([a-zA-Z0-9_$]+|"[^"]*"|'[^']*'|\s+|[(){},.;:[\]])/g)
											.filter(Boolean);

										parts.forEach((part, j) => {
											const trimmed = part.trim();

											// Keywords
											if (
												[
													"import",
													"from",
													"export",
													"const",
													"return",
													"async",
													"await",
													"function",
													"if",
													"else",
												].includes(trimmed)
											) {
												tokens.push(
													<span key={j} className="text-purple-400">
														{part}
													</span>,
												);
											}
											// Functions & Special Rivet Terms
											else if (
												["actor", "broadcast", "deathmatch", "isValidMove"].includes(trimmed)
											) {
												tokens.push(
													<span key={j} className="text-blue-400">
														{part}
													</span>,
												);
											}
											// Object Keys / Properties / Methods
											else if (
												[
													"state",
													"actions",
													"players",
													"scores",
													"map",
													"join",
													"move",
													"connectionId",
													"name",
													"hp",
													"pos",
													"x",
													"y",
													"id",
												].includes(trimmed)
											) {
												tokens.push(
													<span key={j} className="text-blue-300">
														{part}
													</span>,
												);
											}
											// Strings
											else if (part.startsWith('"') || part.startsWith("'")) {
												tokens.push(
													<span key={j} className="text-[#FF4500]">
														{part}
													</span>,
												);
											}
											// Numbers
											else if (!isNaN(Number(trimmed)) && trimmed !== "") {
												tokens.push(
													<span key={j} className="text-emerald-400">
														{part}
													</span>,
												);
											}
											// Default (punctuation, variables like 'c', etc)
											else {
												tokens.push(
													<span key={j} className="text-zinc-300">
														{part}
													</span>,
												);
											}
										});

										if (comment) {
											tokens.push(
												<span key="comment" className="text-zinc-500">
													{comment}
												</span>,
											);
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

const SolutionCard = ({ title, description, icon: Icon, color = "red" }) => {
	const getColorClasses = (col) => {
		switch (col) {
			case "orange":
				return {
					bg: "bg-orange-500/10",
					text: "text-orange-400",
					hoverBg: "group-hover:bg-orange-500/20",
					border: "border-orange-500/80",
					glow: "rgba(249,115,22,0.1)",
				};
			case "blue":
				return {
					bg: "bg-blue-500/10",
					text: "text-blue-400",
					hoverBg: "group-hover:bg-blue-500/20",
					border: "border-blue-500/80",
					glow: "rgba(59,130,246,0.1)",
				};
			case "purple":
				return {
					bg: "bg-purple-500/10",
					text: "text-purple-400",
					hoverBg: "group-hover:bg-purple-500/20",
					border: "border-purple-500/80",
					glow: "rgba(168,85,247,0.1)",
				};
			case "emerald":
				return {
					bg: "bg-emerald-500/10",
					text: "text-emerald-400",
					hoverBg: "group-hover:bg-emerald-500/20",
					border: "border-emerald-500/80",
					glow: "rgba(16,185,129,0.1)",
				};
			case "red":
				return {
					bg: "bg-red-500/10",
					text: "text-red-400",
					hoverBg: "group-hover:bg-red-500/20",
					border: "border-red-500/80",
					glow: "rgba(239,68,68,0.1)",
				};
			default:
				return {
					bg: "bg-red-500/10",
					text: "text-red-400",
					hoverBg: "group-hover:bg-red-500/20",
					border: "border-red-500/80",
					glow: "rgba(239,68,68,0.1)",
				};
		}
	};
	const c = getColorClasses(color);

	return (
		<div className="group relative overflow-hidden rounded-2xl border border-white/5 bg-black/50 backdrop-blur-sm flex flex-col h-full p-6">
			{/* Top Shine Highlight */}
			<div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent" />

			{/* Soft Glow */}
			<div
				className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none"
				style={{
					background: `radial-gradient(circle at top left, ${c.glow} 0%, transparent 50%)`,
				}}
			/>

			{/* Sharp Edge Highlight (Masked & Shortened) */}
			<div
				className={`absolute top-0 left-0 w-12 h-12 rounded-tl-2xl border-t border-l ${c.border} opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none z-20 [mask-image:linear-gradient(135deg,black_0%,transparent_100%)]`}
			/>

			<div className="flex items-center gap-3 mb-4 relative z-10">
				<div
					className={`p-2 rounded ${c.bg} ${c.text} ${c.hoverBg} transition-colors duration-500`}
				>
					<Icon className="w-5 h-5" />
				</div>
				<h3 className="text-lg font-medium text-white tracking-tight">{title}</h3>
			</div>
			<p className="text-sm text-zinc-400 leading-relaxed relative z-10 flex-grow">
				{description}
			</p>
		</div>
	);
};

// --- Page Sections ---
const Hero = () => (
	<section className="relative pt-32 pb-20 md:pt-48 md:pb-32 overflow-hidden">
		<div className="absolute top-0 left-1/2 -translate-x-1/2 w-[1000px] h-[500px] bg-red-500/[0.03] blur-[100px] rounded-full pointer-events-none" />

		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<Badge text="Multiplayer Backend" color="red" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="text-5xl md:text-7xl font-medium text-white tracking-tight leading-[1.1] mb-6"
					>
						Game Servers. <br />
						<span className="text-red-400">Serverless.</span>
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg md:text-xl text-zinc-400 leading-relaxed mb-8 max-w-lg"
					>
						Launch an authoritative game server for every match instantly. Scale to millions of
						concurrent players without managing fleets or Kubernetes.
					</motion.p>

					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="flex flex-col sm:flex-row items-center gap-4"
					>
						<button className="w-full sm:w-auto h-12 px-8 rounded-full bg-white text-black font-semibold hover:bg-zinc-200 transition-colors flex items-center justify-center gap-2">
							Deploy Match
							<ArrowRight className="w-4 h-4" />
						</button>
						<button className="w-full sm:w-auto h-12 px-8 rounded-full border border-zinc-800 text-zinc-300 font-medium hover:text-white hover:border-zinc-600 transition-colors flex items-center justify-center gap-2 bg-black">
							<Play className="w-4 h-4" />
							See Examples
						</button>
					</motion.div>
				</div>

				<div className="flex-1 w-full max-w-xl">
					<div className="relative">
						<div className="absolute -inset-1 bg-gradient-to-r from-red-500/20 to-orange-500/20 rounded-xl blur opacity-40" />
						<CodeBlock
							fileName="match_handler.ts"
							code={`import { actor } from "rivetkit";

export const deathmatch = actor({
  state: { players: {}, scores: {}, map: 'arena_1' },

  actions: {
    join: (c, { name }) => {
      c.state.players[c.connectionId] = { name, hp: 100, pos: {x:0, y:0} };
      c.broadcast("player_join", c.state.players);
    },

    move: (c, { x, y }) => {
      // Authoritative movement validation
      if (isValidMove(c.state.map, x, y)) {
         c.state.players[c.connectionId].pos = { x, y };
         c.broadcast("update", { id: c.connectionId, x, y });
      }
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

const GameLoopArchitecture = () => {
	const [tick, setTick] = useState(0);

	useEffect(() => {
		const interval = setInterval(() => {
			setTick((t) => t + 1);
		}, 500);
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
						className="text-3xl md:text-4xl font-medium text-white mb-6"
					>
						The Game Loop
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-zinc-400 max-w-2xl text-lg leading-relaxed"
					>
						Traditional serverless functions die after a request. Rivet Actors stay alive,
						maintaining the game state in memory and ticking the simulation as long as players are
						connected.
					</motion.p>
				</div>

				<div className="grid lg:grid-cols-2 gap-12 items-center">
					{/* Visualization */}
					<div className="relative h-80 rounded-2xl border border-white/10 bg-zinc-900/20 flex items-center justify-center overflow-hidden p-8">
						{/* Central Actor (Server) */}
						<div className="relative z-10 flex flex-col items-center">
							<div className="w-32 h-32 rounded-full border-2 border-red-500 bg-black flex flex-col items-center justify-center relative shadow-[0_0_40px_rgba(239,68,68,0.3)]">
								<div className="absolute inset-0 rounded-full border border-red-500/30 animate-ping opacity-50" />
								<Server className="w-8 h-8 text-red-500 mb-2" />
								<div className="text-[10px] font-mono text-zinc-400">TICK: {tick}</div>
							</div>

							{/* Packets */}
							<div className="absolute w-full h-full flex items-center justify-center pointer-events-none">
								{/* Outgoing Update (Broadcast) */}
								<div
									className={`absolute w-48 h-48 border border-red-500/50 rounded-full transition-all duration-500 ${
										tick % 2 === 0 ? "scale-100 opacity-100" : "scale-50 opacity-0"
									}`}
								/>
							</div>
						</div>

						{/* Clients */}
						<div className="absolute inset-0">
							{/* Client 1 */}
							<div className="absolute top-1/2 left-8 -translate-y-1/2 flex flex-col items-center gap-2">
								<div className="w-10 h-10 bg-zinc-800 rounded border border-zinc-600 flex items-center justify-center">
									<Gamepad2 className="w-5 h-5 text-white" />
								</div>
								<span className="text-[10px] font-mono text-zinc-500">Player 1</span>
								{/* Incoming Input */}
								<div
									className={`absolute left-full top-1/2 w-2 h-2 bg-blue-400 rounded-full transition-all duration-300 ${
										tick % 2 !== 0 ? "translate-x-16 opacity-0" : "translate-x-0 opacity-100"
									}`}
								/>
							</div>

							{/* Client 2 */}
							<div className="absolute top-1/2 right-8 -translate-y-1/2 flex flex-col items-center gap-2">
								<div className="w-10 h-10 bg-zinc-800 rounded border border-zinc-600 flex items-center justify-center">
									<Gamepad2 className="w-5 h-5 text-white" />
								</div>
								<span className="text-[10px] font-mono text-zinc-500">Player 2</span>
								{/* Incoming Input P2 */}
								<div
									className={`absolute right-full top-1/2 w-2 h-2 bg-green-400 rounded-full transition-all duration-300 ${
										tick % 2 === 0 ? "-translate-x-16 opacity-0" : "translate-x-0 opacity-100"
									}`}
								/>
							</div>
						</div>

						{/* Server Console */}
						<div className="absolute bottom-4 left-4 right-4 h-16 bg-black/80 border border-white/10 rounded font-mono text-[10px] p-2 text-zinc-400 overflow-hidden backdrop-blur-md">
							<div className="text-red-400 opacity-50">server.tick({tick - 1})</div>
							<div className="text-red-500">
								server.tick({tick}) &gt; Broadcasting State Snapshot
							</div>
							<div className="opacity-80">
								{tick % 2 !== 0 ? (
									<span className="text-blue-400">
										&lt; Player 1 Input: Move(x: 12, y: 40)
									</span>
								) : (
									<span className="text-green-400">
										&lt; Player 2 Input: Attack(target: P1)
									</span>
								)}
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
								<Cpu className="w-5 h-5 text-red-400" />
								Authoritative Logic
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								Run your game logic (movement, hit detection, inventory) on the server to prevent
								cheating. The Actor is the single source of truth.
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
								<Wifi className="w-5 h-5 text-blue-400" />
								Instant Connectivity
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								Clients connect directly to the specific Actor instance hosting their match via
								WebSockets. No database polling latency.
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
								<Database className="w-5 h-5 text-orange-400" />
								Persistence on Exit
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								When the match ends, the final state (scores, loot, XP) is automatically saved to
								disk.
							</p>
						</motion.div>
					</div>
				</div>
			</div>
		</section>
	);
};

const GameFeatures = () => {
	const features = [
		{
			title: "Lobby Management",
			description:
				"Create persistent lobby actors that hold players before a match starts. Handle chat, loadouts, and ready states.",
			icon: Users,
			color: "red",
		},
		{
			title: "Matchmaking",
			description:
				"Use a singleton 'Matchmaker' actor to queue players and spawn new Match actors when groups are formed.",
			icon: Target,
			color: "blue",
		},
		{
			title: "Turn-Based Games",
			description:
				"Perfect for card games or board games. Actors can sleep for days between turns without incurring compute costs.",
			icon: Clock,
			color: "orange",
		},
		{
			title: "Leaderboards",
			description:
				"High-throughput counters and sorting in memory. Update scores instantly without hammering a database.",
			icon: Trophy,
			color: "purple",
		},
		{
			title: "Economy & Inventory",
			description:
				"Transactional state for trading items or currency. Ensure no item duplication glitches with serialized execution.",
			icon: CreditCard,
			color: "emerald",
		},
		{
			title: "Spectator Mode",
			description:
				"Allow thousands of users to subscribe to a match actor to watch real-time updates without affecting player latency.",
			icon: Eye,
			color: "red",
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
						className="text-3xl md:text-5xl font-medium text-white mb-6"
					>
						Built for Multiplayer
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-zinc-400 text-lg"
					>
						Infrastructure primitives that understand the needs of modern games.
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
					<Badge text="Case Study" color="red" />
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6"
					>
						Real-Time Strategy (RTS)
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg text-zinc-400 mb-8 leading-relaxed"
					>
						A persistent world 4X strategy game where thousands of players move armies on a shared
						map.
					</motion.p>
					<motion.ul
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="space-y-4"
					>
						{[
							"Sharding: Map divided into hex grids, each controlled by an Actor",
							"Fog of War: Calculated on server, only visible units sent to client",
							"Persistence: Game state survives server updates seamlessly",
						].map((item, i) => (
							<li key={i} className="flex items-center gap-3 text-zinc-300">
								<div className="w-5 h-5 rounded-full bg-red-500/20 flex items-center justify-center">
									<Check className="w-3 h-3 text-red-400" />
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
					<div className="absolute inset-0 bg-gradient-to-r from-red-500/20 to-transparent rounded-2xl blur-2xl" />
					<div className="relative rounded-2xl border border-white/10 bg-zinc-900 p-6 shadow-2xl">
						<div className="flex items-center justify-between mb-6 border-b border-white/5 pb-4">
							<div className="flex items-center gap-3">
								<div className="w-8 h-8 rounded bg-red-500/20 flex items-center justify-center">
									<Sword className="w-5 h-5 text-red-400" />
								</div>
								<div>
									<div className="text-sm font-medium text-white">Sector: Alpha-9</div>
									<div className="text-xs text-zinc-500">Units: 4,291 Active</div>
								</div>
							</div>
							<div className="px-2 py-1 rounded bg-red-500/10 text-red-400 text-xs border border-red-500/20">
								Live
							</div>
						</div>
						<div className="space-y-4 text-sm font-mono">
							<div className="flex justify-between items-center text-zinc-500 text-xs">
								<span>Tick Rate</span>
								<span>20Hz</span>
							</div>
							<div className="w-full bg-zinc-800 rounded-full h-1.5 overflow-hidden">
								<div
									className="bg-red-500 h-1.5 rounded-full animate-[pulse_2s_infinite]"
									style={{ width: "92%" }}
								/>
							</div>
							<div className="p-3 rounded bg-zinc-950 border border-white/5 text-zinc-400">
								Combat resolved in Grid[44,12]. 12 units lost. Updating clients...
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
				className="text-3xl md:text-5xl font-medium text-white mb-12"
			>
				Integrates with your engine
			</motion.h2>
			<div className="flex flex-wrap justify-center gap-4">
				{["Unity", "Unreal Engine", "Godot", "Phaser", "Three.js", "PlayCanvas"].map(
					(tech, i) => (
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
					),
				)}
			</div>
		</div>
	</section>
);

export default function GameServersPage() {
	return (
		<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-red-500/30 selection:text-red-200">
			<main>
				<Hero />
				<GameLoopArchitecture />
				<GameFeatures />
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
							Launch day ready.
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-lg text-zinc-400 mb-10 leading-relaxed"
						>
							Focus on the gameplay. Let Rivet handle the state, scaling, and persistence.
						</motion.p>
						<motion.div
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.2 }}
							className="flex flex-col sm:flex-row items-center justify-center gap-4"
						>
							<button className="w-full sm:w-auto px-8 py-4 rounded-full bg-white text-black font-semibold hover:bg-zinc-200 transition-all transform hover:-translate-y-1">
								Start Building
							</button>
							<button className="w-full sm:w-auto px-8 py-4 rounded-full bg-zinc-900 text-white border border-zinc-800 font-medium hover:bg-zinc-800 transition-all">
								Read the Docs
							</button>
						</motion.div>
					</div>
				</section>
			</main>
		</div>
	);
}

