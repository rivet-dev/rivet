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
const Badge = ({ text }: { text: string }) => (
	<div className="inline-flex items-center gap-2 rounded-full border border-white/10 px-3 py-1 text-xs text-zinc-400 mb-6">
		<span className="h-1.5 w-1.5 rounded-full bg-[#FF4500]" />
		{text}
	</div>
);

const CodeBlock = ({ code, fileName = "match.ts" }: { code: string; fileName?: string }) => {
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
			else if (["state", "actions", "broadcast", "c", "match", "players", "gameState", "join", "leave", "update"].includes(trimmed)) {
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

// --- Simple Feature Card ---
const FeatureItem = ({ title, description, icon: Icon }: { title: string; description: string; icon: typeof Database }) => (
	<div className="border-t border-white/10 pt-6">
		<div className="mb-3 text-zinc-500">
			<Icon className="h-4 w-4" />
		</div>
		<h3 className="text-sm font-normal text-white mb-1">{title}</h3>
		<p className="text-sm leading-relaxed text-zinc-500">{description}</p>
	</div>
);

// --- Page Sections ---
const Hero = () => (
	<section className="relative overflow-hidden pb-20 pt-32 md:pb-32 md:pt-48">
		<div className="mx-auto max-w-7xl px-6">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<Badge text="Multiplayer Backend" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="mb-6 text-4xl font-normal leading-[1.1] tracking-tight text-white md:text-6xl"
					>
						Serverless Game Servers
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="mb-8 max-w-lg text-base leading-relaxed text-zinc-500"
					>
						Launch an authoritative game server for every match instantly. Scale to millions of
						concurrent players without managing fleets or Kubernetes.
					</motion.p>

					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="flex flex-col sm:flex-row items-center gap-3"
					>
						<a href="/docs" className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200">
							Start Building
							<ArrowRight className="h-4 w-4" />
						</a>
						<a href="https://github.com/rivet-dev/rivet/tree/main/examples/chat-room" className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white">
							<Play className="h-4 w-4" />
							See Examples
						</a>
					</motion.div>
				</div>

				<div className="flex-1 w-full max-w-xl">
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
		<section className="border-t border-white/10 py-48">
			<div className="mx-auto max-w-7xl px-6">
				<div className="mb-12">
					<h2 className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl">
						The Game Loop
					</h2>
					<p className="max-w-2xl text-base leading-relaxed text-zinc-500">
						Traditional serverless functions die after a request. Rivet Actors stay alive,
						maintaining the game state in memory and ticking the simulation as long as players are
						connected.
					</p>
				</div>

				<div className="grid lg:grid-cols-2 gap-12 items-center">
					{/* Visualization */}
					<div className="relative h-80 rounded-lg border border-white/10 bg-black flex items-center justify-center overflow-hidden p-8">
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
			description: "Create persistent lobby actors that hold players before a match starts. Handle chat, loadouts, and ready states.",
			icon: Users,
		},
		{
			title: "Matchmaking",
			description: "Use a singleton 'Matchmaker' actor to queue players and spawn new Match actors when groups are formed.",
			icon: Target,
		},
		{
			title: "Turn-Based Games",
			description: "Perfect for card games or board games. Actors can sleep for days between turns without incurring compute costs.",
			icon: Clock,
		},
		{
			title: "Leaderboards",
			description: "High-throughput counters and sorting in memory. Update scores instantly without hammering a database.",
			icon: Trophy,
		},
		{
			title: "Economy & Inventory",
			description: "Transactional state for trading items or currency. Ensure no item duplication glitches with serialized execution.",
			icon: CreditCard,
		},
		{
			title: "Spectator Mode",
			description: "Allow thousands of users to subscribe to a match actor to watch real-time updates without affecting player latency.",
			icon: Eye,
		},
	];

	return (
		<section className="border-t border-white/10 py-48">
			<div className="mx-auto max-w-7xl px-6">
				<div className="flex flex-col gap-12">
					<div className="max-w-xl">
						<h2 className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl">Built for Multiplayer</h2>
						<p className="text-base leading-relaxed text-zinc-500">Infrastructure primitives that understand the needs of modern games.</p>
					</div>

					<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
						{features.map((feat, idx) => (
							<FeatureItem key={idx} {...feat} />
						))}
					</div>
				</div>
			</div>
		</section>
	);
};

const UseCases = () => (
	<section className="border-t border-white/10 py-48">
		<div className="mx-auto max-w-7xl px-6">
			<div className="grid md:grid-cols-2 gap-16 items-center">
				<div>
					<Badge text="Case Study" />
					<h2 className="mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl">
						Real-Time Strategy (RTS)
					</h2>
					<p className="mb-8 text-base leading-relaxed text-zinc-500">
						A persistent world 4X strategy game where thousands of players move armies on a shared
						map.
					</p>
					<ul className="space-y-4">
						{[
							"Sharding: Map divided into hex grids, each controlled by an Actor",
							"Fog of War: Calculated on server, only visible units sent to client",
							"Persistence: Game state survives server updates seamlessly",
						].map((item, i) => (
							<li key={i} className="flex items-center gap-3 text-sm text-zinc-300">
								<div className="flex h-5 w-5 items-center justify-center rounded-full border border-white/10 bg-white/5">
									<Check className="h-3 w-3 text-[#FF4500]" />
								</div>
								{item}
							</li>
						))}
					</ul>
				</div>
				<div className="relative rounded-lg border border-white/10 bg-black p-6">
					<div className="flex items-center justify-between mb-6 border-b border-white/5 pb-4">
						<div className="flex items-center gap-3">
							<div className="flex h-8 w-8 items-center justify-center rounded-lg border border-white/10 bg-white/5">
								<Sword className="h-4 w-4 text-white" />
							</div>
							<div>
								<div className="text-sm font-medium text-white">Sector: Alpha-9</div>
								<div className="text-xs text-zinc-500">Units: 4,291 Active</div>
							</div>
						</div>
						<div className="rounded border border-[#FF4500]/20 bg-[#FF4500]/10 px-2 py-1 text-xs text-[#FF4500]">
							Live
						</div>
					</div>
					<div className="space-y-4 font-mono text-sm">
						<div className="flex items-center justify-between text-xs text-zinc-500">
							<span>Tick Rate</span>
							<span>20Hz</span>
						</div>
						<div className="h-1.5 w-full overflow-hidden rounded-full bg-zinc-800">
							<div className="h-1.5 w-[92%] animate-pulse rounded-full bg-[#FF4500]" />
						</div>
						<div className="rounded border border-white/5 bg-zinc-900 p-3 text-zinc-400">
							Combat resolved in Grid[44,12]. 12 units lost. Updating clients...
						</div>
					</div>
				</div>
			</div>
		</div>
	</section>
);

const Ecosystem = () => (
	<section className="border-t border-white/10 py-48">
		<div className="mx-auto max-w-7xl px-6 text-center">
			<h2 className="mb-12 text-2xl font-normal tracking-tight text-white md:text-4xl">
				Integrates with your engine
			</h2>
			<div className="flex flex-wrap justify-center gap-3">
				{["Unity", "Unreal Engine", "Godot", "Phaser", "Three.js", "PlayCanvas"].map((tech) => (
					<div
						key={tech}
						className="rounded-md border border-white/5 px-2 py-1 text-xs text-zinc-400 transition-colors hover:border-white/20 hover:text-white"
					>
						{tech}
					</div>
				))}
			</div>
		</div>
	</section>
);

export default function GameServersPage() {
	return (
		<div className="min-h-screen bg-black font-sans text-zinc-300 selection:bg-[#FF4500]/30 selection:text-orange-200">
			<main>
				<Hero />
				<GameLoopArchitecture />
				<GameFeatures />
				<UseCases />
				<Ecosystem />

				{/* CTA Section */}
				<section className="border-t border-white/10 py-48">
					<div className="mx-auto max-w-3xl px-6 text-center">
						<h2 className="mb-4 text-2xl font-normal tracking-tight text-white md:text-4xl">
							Launch day ready.
						</h2>
						<p className="mb-8 text-base leading-relaxed text-zinc-500">
							Focus on the gameplay. Let Rivet handle the state, scaling, and persistence.
						</p>
						<div className="flex flex-col items-center justify-center gap-3 sm:flex-row">
							<a href="/docs" className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200">
								Start Building
								<ArrowRight className="h-4 w-4" />
							</a>
							<a href="/docs/actors" className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white">
								Read the Docs
							</a>
						</div>
					</div>
				</section>
			</main>
		</div>
	);
}
