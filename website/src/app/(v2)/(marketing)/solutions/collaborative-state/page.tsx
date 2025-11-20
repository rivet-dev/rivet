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
	MousePointer2,
	Share2,
	Edit3,
} from "lucide-react";
import { motion } from "framer-motion";

// --- Shared Design Components ---
const Badge = ({ text, color = "blue" }) => {
	const colorClasses = {
		orange: "text-orange-400 border-orange-500/20 bg-orange-500/10",
		blue: "text-blue-400 border-blue-500/20 bg-blue-500/10",
		purple: "text-purple-400 border-purple-500/20 bg-purple-500/10",
		emerald: "text-emerald-400 border-emerald-500/20 bg-emerald-500/10",
		green: "text-green-400 border-green-500/20 bg-green-500/10",
	};

	return (
		<div
			className={`inline-flex items-center gap-2 px-3 py-1 rounded-full border text-xs font-medium mb-8 transition-colors cursor-default ${colorClasses[color]}`}
		>
			<span className={`w-1.5 h-1.5 rounded-full ${color === "orange" ? "bg-orange-400" : color === "blue" ? "bg-blue-400" : color === "purple" ? "bg-purple-400" : color === "emerald" ? "bg-emerald-400" : "bg-green-400"} animate-pulse`} />
			{text}
		</div>
	);
};

const CodeBlock = ({ code, fileName = "room.ts" }) => {
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
											else if (["actor", "broadcast", "spawn", "rpc", "applyPatch", "connectionId"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-400">{part}</span>);
											}
											// Object Keys / Properties / Methods
											else if (["state", "actions", "content", "cursors", "update", "patch", "moveCursor", "x", "y", "presence"].includes(trimmed)) {
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

// --- Refined Collaboration Card matching landing page style with color highlights ---
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
			case "green":
				return {
					bg: "bg-green-500/10",
					text: "text-green-400",
					hoverBg: "group-hover:bg-green-500/20",
					hoverShadow: "group-hover:shadow-[0_0_15px_rgba(34,197,94,0.5)]",
					border: "border-green-500",
					glow: "rgba(34,197,94,0.15)",
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
		<div className="absolute top-0 left-1/2 -translate-x-1/2 w-[1000px] h-[500px] bg-blue-500/[0.03] blur-[100px] rounded-full pointer-events-none" />

		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<Badge text="Real-time Sync" color="blue" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="text-5xl md:text-7xl font-medium text-white tracking-tight leading-[1.1] mb-6"
					>
						Multiplayer by <br />
						<span className="text-blue-400">Default.</span>
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg md:text-xl text-zinc-400 leading-relaxed mb-8 max-w-lg"
					>
						Stop managing WebSocket fleets. Rivet Actors give you instant, stateful rooms for collaborative documents, whiteboards, and chat.
					</motion.p>
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="flex flex-col sm:flex-row items-center gap-4"
					>
						<button className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black shadow-sm hover:bg-zinc-200 transition-colors gap-2">
							Create Room
							<ArrowRight className="w-4 h-4" />
						</button>
					</motion.div>
				</div>
				<div className="flex-1 w-full max-w-xl">
					<div className="relative">
						<div className="absolute -inset-1 bg-gradient-to-r from-blue-500/20 to-green-500/20 rounded-xl blur opacity-40" />
						<CodeBlock
							fileName="document_room.ts"
							code={`import { actor } from "rivetkit";

export const docRoom = actor({
  // State held in RAM for < 1ms access
  state: { content: "", cursors: {} },

  actions: {
    // Handle keypresses instantly
    update: (c, patch) => {
      c.state.content = applyPatch(c.state.content, patch);
      
      // Broadcast to all other clients in room
      c.broadcast("patch", patch);
    },

    // Ephemeral state for presence
    moveCursor: (c, { x, y }) => {
      c.state.cursors[c.connectionId] = { x, y };
      c.broadcast("presence", c.state.cursors);
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

const RoomArchitecture = () => {
	const [cursor1, setCursor1] = useState({ x: 30, y: 40 });
	const [cursor2, setCursor2] = useState({ x: 70, y: 60 });

	useEffect(() => {
		const interval = setInterval(() => {
			setCursor1({ x: 30 + Math.sin(Date.now() / 1000) * 20, y: 40 + Math.cos(Date.now() / 1000) * 10 });
			setCursor2({ x: 70 + Math.cos(Date.now() / 800) * 15, y: 60 + Math.sin(Date.now() / 800) * 15 });
		}, 50);
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
						Room-Based Architecture
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-zinc-400 max-w-2xl text-lg leading-relaxed"
					>
						Every document or session gets its own dedicated Actor. This isolates state, prevents database contention, and guarantees order of operations.
					</motion.p>
				</div>

				<div className="grid lg:grid-cols-2 gap-12 items-center">
					{/* Interactive Diagram */}
					<div className="relative h-80 rounded-2xl border border-white/10 bg-zinc-900/20 flex items-center justify-center overflow-hidden">
						{/* The Room (Actor) */}
						<div className="relative w-64 h-48 rounded-xl border border-blue-500/50 bg-blue-500/5 backdrop-blur-sm flex flex-col items-center justify-center z-10">
							<div className="absolute -top-3 left-4 bg-blue-500 text-black text-[10px] font-medium px-2 py-0.5 rounded">ACTOR: room-8392</div>

							{/* Simulated Doc */}
							<div className="w-48 h-32 bg-zinc-900 border border-white/10 rounded p-3 relative">
								<div className="space-y-2">
									<div className="h-2 w-3/4 bg-zinc-700 rounded animate-pulse" />
									<div className="h-2 w-1/2 bg-zinc-700 rounded animate-pulse delay-75" />
									<div className="h-2 w-5/6 bg-zinc-700 rounded animate-pulse delay-150" />
								</div>
								{/* Virtual Cursors */}
								<div className="absolute w-3 h-3" style={{ top: `${cursor1.y}%`, left: `${cursor1.x}%`, transition: "all 0.1s linear" }}>
									<MousePointer2 className="w-3 h-3 text-green-400 fill-green-400" />
									<div className="absolute top-4 left-0 bg-green-500/90 text-black text-[8px] px-1 rounded">User A</div>
								</div>
								<div className="absolute w-3 h-3" style={{ top: `${cursor2.y}%`, left: `${cursor2.x}%`, transition: "all 0.1s linear" }}>
									<MousePointer2 className="w-3 h-3 text-purple-400 fill-purple-400" />
									<div className="absolute top-4 left-0 bg-purple-500/90 text-black text-[8px] px-1 rounded">User B</div>
								</div>
							</div>
						</div>

						{/* Connection Lines (Decorative) */}
						<div className="absolute bottom-0 left-1/4 w-[1px] h-16 bg-gradient-to-t from-transparent to-green-500/50" />
						<div className="absolute bottom-0 right-1/4 w-[1px] h-16 bg-gradient-to-t from-transparent to-purple-500/50" />
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
								<MousePointer2 className="w-5 h-5 text-green-400" />
								Instant Presence
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								Broadcast cursor positions and selection states to 100+ users in the same room with &lt;10ms latency.
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
								<Database className="w-5 h-5 text-blue-400" />
								Authoritative State
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								The Actor holds the "source of truth" in memory. Resolve conflicts on the server or relay operations for client-side CRDT merging.
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
								<Server className="w-5 h-5 text-purple-400" />
								Connection Limits
							</h3>
							<p className="text-zinc-400 text-sm leading-relaxed">
								Automatically scales to handle thousands of concurrent active rooms. Each room hibernates when the last user leaves.
							</p>
						</motion.div>
					</div>
				</div>
			</div>
		</section>
	);
};

const CollaborationFeatures = () => {
	const features = [
		{
			title: "WebSockets Included",
			description: "No need for Pusher or separate socket servers. Actors speak WebSocket natively. Just connect() and listen.",
			icon: Wifi,
			color: "green",
		},
		{
			title: "Broadcast & Pub/Sub",
			description: "Send a message to everyone in the room, or target specific users. Built-in channels for effortless event routing.",
			icon: Share2,
			color: "blue",
		},
		{
			title: "Ephemeral Storage",
			description: "Perfect for 'who is typing' indicators or selection highlights that don't need to be saved to the database.",
			icon: Zap,
			color: "orange",
		},
		{
			title: "Conflict Resolution",
			description: "Run logic on the server to validate moves or merge edits before they are broadcast to other players.",
			icon: Workflow,
			color: "purple",
		},
		{
			title: "History & Replay",
			description: "Keep a running log of actions in memory. Allow users to undo/redo or replay the session history.",
			icon: RefreshCw,
			color: "blue",
		},
		{
			title: "Yjs & Automerge",
			description: "A perfect host for CRDT backends. Store the encoded document state in the Actor and sync changes effortlessly.",
			icon: FileText,
			color: "green",
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
						The Engine for Collaboration
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-zinc-400"
					>
						Primitives designed for high-concurrency, low-latency interactive apps.
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
					<Badge text="Ideal For" color="blue" />
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-3xl md:text-5xl font-medium text-white mb-6 tracking-tight"
					>
						Infinite Canvases
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-lg text-zinc-400 mb-8 leading-relaxed"
					>
						Build the next Figma or Miro. Store thousands of vector objects in memory and stream updates only for the viewport.
					</motion.p>
					<motion.ul
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="space-y-4"
					>
						{[
							"Spatial Indexing: Query objects by x/y coordinates",
							"Delta Compression: Only send changed attributes",
							"Locking: Prevent two users from moving the same object",
						].map((item, i) => (
							<li key={i} className="flex items-center gap-3 text-zinc-300">
								<div className="w-5 h-5 rounded-full bg-blue-500/20 flex items-center justify-center">
									<Check className="w-3 h-3 text-blue-400" />
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
					<div className="absolute inset-0 bg-gradient-to-r from-blue-500/20 to-purple-500/20 rounded-2xl blur-2xl" />
					<div className="relative rounded-2xl border border-white/10 bg-zinc-900/80 p-2 shadow-2xl aspect-video flex items-center justify-center overflow-hidden">
						{/* Grid Pattern */}
						<div className="absolute inset-0 opacity-20 bg-[linear-gradient(#fff_1px,transparent_1px),linear-gradient(90deg,#fff_1px,transparent_1px)] bg-[size:20px_20px]" />

						{/* Shapes */}
						<div className="absolute top-1/3 left-1/4 w-16 h-16 border-2 border-blue-400 bg-blue-500/20 rounded transform -rotate-12" />
						<div className="absolute bottom-1/3 right-1/3 w-20 h-20 border-2 border-purple-400 bg-purple-500/20 rounded-full" />

						{/* Cursor interacting */}
						<div className="absolute top-1/3 left-1/4 translate-x-12 translate-y-12">
							<MousePointer2 className="w-4 h-4 text-white fill-black" />
							<div className="bg-blue-500 text-black text-[10px] px-1 rounded ml-2">Sarah</div>
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
				Integrates with
			</motion.h2>
			<div className="flex flex-wrap justify-center gap-4">
				{["Y.js", "Automerge", "Prosemirror", "Tldraw", "Excalidraw", "Liveblocks Client"].map((tech, i) => (
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

export default function CollaborativeStatePage() {
	return (
		<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-blue-500/30 selection:text-blue-200">
			<main>
				<Hero />
				<RoomArchitecture />
				<CollaborationFeatures />
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
							Ready to go multiplayer?
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-lg text-zinc-400 mb-10 leading-relaxed"
						>
							Build the collaborative features your users expect, without the infrastructure headache.
						</motion.p>
						<motion.div
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.2 }}
							className="flex flex-col sm:flex-row items-center justify-center gap-4"
						>
							<button className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black shadow-sm hover:bg-zinc-200 transition-colors">
								Start for Free
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

