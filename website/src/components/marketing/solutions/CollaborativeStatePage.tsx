"use client";

import { useState, useEffect } from "react";
import {
	Zap,
	ArrowRight,
	Database,
	Check,
	RefreshCw,
	Wifi,
	FileText,
	Workflow,
	Server,
	MousePointer2,
	Share2,
} from "lucide-react";
import { motion } from "framer-motion";

// --- Shared Design Components ---
const Badge = ({ text }: { text: string }) => (
	<div className="inline-flex items-center gap-2 rounded-full border border-white/10 px-3 py-1 text-xs text-zinc-400 mb-6">
		<span className="h-1.5 w-1.5 rounded-full bg-[#FF4500]" />
		{text}
	</div>
);

const CodeBlock = ({ code, fileName = "room.ts" }: { code: string; fileName?: string }) => {
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
			else if (["state", "actions", "broadcast", "c", "room", "users", "cursor", "position", "update"].includes(trimmed)) {
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

// --- Feature Item Component matching landing page style ---
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
	<section className="relative pt-32 pb-20 md:pt-48 md:pb-32 overflow-hidden">
		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<Badge text="Real-time Sync" />

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="text-4xl md:text-6xl font-normal text-white tracking-tight leading-[1.1] mb-6"
					>
						Multiplayer by Default
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-base text-zinc-500 leading-relaxed mb-8 max-w-lg"
					>
						Stop managing WebSocket fleets. Rivet Actors give you instant, stateful rooms for collaborative documents, whiteboards, and chat.
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
						<a href="/templates/cursors" className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white gap-2">
							View Example
						</a>
					</motion.div>
				</div>
				<div className="flex-1 w-full max-w-xl">
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
      c.state.cursors[c.conn.id] = { x, y };
      c.broadcast("presence", c.state.cursors);
    }
  }
});`}
					/>
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
						Room-Based Architecture
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-base leading-relaxed text-zinc-500 max-w-2xl"
					>
						Every document or session gets its own dedicated Actor. This isolates state, prevents database contention, and guarantees order of operations.
					</motion.p>
				</div>

				<div className="grid lg:grid-cols-2 gap-12 items-center">
					{/* Interactive Diagram */}
					<div className="relative h-80 rounded-lg border border-white/10 bg-black flex items-center justify-center overflow-hidden">
						{/* The Room (Actor) */}
						<div className="relative w-64 h-48 rounded-xl border border-white/20 bg-white/5 backdrop-blur-sm flex flex-col items-center justify-center z-10">
							<div className="absolute -top-3 left-4 bg-[#FF4500] text-black text-[10px] font-medium px-2 py-0.5 rounded">ACTOR: room-8392</div>

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
							className="border-t border-white/10 pt-6"
						>
							<div className="mb-3 text-zinc-500">
								<MousePointer2 className="h-4 w-4" />
							</div>
							<h3 className="text-sm font-normal text-white mb-1">
								Instant Presence
							</h3>
							<p className="text-sm leading-relaxed text-zinc-500">
								Broadcast cursor positions and selection states to 100+ users in the same room with &lt;10ms latency.
							</p>
						</motion.div>
						<motion.div
							initial={{ opacity: 0, x: 20 }}
							whileInView={{ opacity: 1, x: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="border-t border-white/10 pt-6"
						>
							<div className="mb-3 text-zinc-500">
								<Database className="h-4 w-4" />
							</div>
							<h3 className="text-sm font-normal text-white mb-1">
								Authoritative State
							</h3>
							<p className="text-sm leading-relaxed text-zinc-500">
								The Actor holds the "source of truth" in memory. Resolve conflicts on the server or relay operations for client-side CRDT merging.
							</p>
						</motion.div>
						<motion.div
							initial={{ opacity: 0, x: 20 }}
							whileInView={{ opacity: 1, x: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.2 }}
							className="border-t border-white/10 pt-6"
						>
							<div className="mb-3 text-zinc-500">
								<Server className="h-4 w-4" />
							</div>
							<h3 className="text-sm font-normal text-white mb-1">
								Connection Limits
							</h3>
							<p className="text-sm leading-relaxed text-zinc-500">
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
		},
		{
			title: "Broadcast & Pub/Sub",
			description: "Send a message to everyone in the room, or target specific users. Built-in channels for effortless event routing.",
			icon: Share2,
		},
		{
			title: "Ephemeral Storage",
			description: "Perfect for 'who is typing' indicators or selection highlights that don't need to be saved to the database.",
			icon: Zap,
		},
		{
			title: "Conflict Resolution",
			description: "Run logic on the server to validate moves or merge edits before they are broadcast to other players.",
			icon: Workflow,
		},
		{
			title: "History & Replay",
			description: "Keep a running log of actions in memory. Allow users to undo/redo or replay the session history.",
			icon: RefreshCw,
		},
		{
			title: "Yjs & Automerge",
			description: "A perfect host for CRDT backends. Store the encoded document state in the Actor and sync changes effortlessly.",
			icon: FileText,
		},
	];

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
						The Engine for Collaboration
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-base leading-relaxed text-zinc-500"
					>
						Primitives designed for high-concurrency, low-latency interactive apps.
					</motion.p>
				</div>

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

const UseCases = () => (
	<section className="border-t border-white/10 py-48">
		<div className="max-w-7xl mx-auto px-6">
			<div className="grid md:grid-cols-2 gap-16 items-center">
				<div>
					<Badge text="Ideal For" />
					<motion.h2
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5 }}
						className="text-2xl font-normal tracking-tight text-white md:text-4xl mb-2"
					>
						Infinite Canvases
					</motion.h2>
					<motion.p
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-base text-zinc-500 mb-8 leading-relaxed"
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
							<li key={i} className="flex items-center gap-3 text-zinc-300 text-sm">
								<Check className="w-4 h-4 text-[#FF4500]" />
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
					<div className="relative rounded-lg border border-white/10 bg-black p-2 aspect-video flex items-center justify-center overflow-hidden">
						{/* Grid Pattern */}
						<div className="absolute inset-0 opacity-10 bg-[linear-gradient(#fff_1px,transparent_1px),linear-gradient(90deg,#fff_1px,transparent_1px)] bg-[size:20px_20px]" />

						{/* Shapes */}
						<div className="absolute top-1/3 left-1/4 w-16 h-16 border-2 border-white/30 bg-white/10 rounded transform -rotate-12" />
						<div className="absolute bottom-1/3 right-1/3 w-20 h-20 border-2 border-white/30 bg-white/10 rounded-full" />

						{/* Cursor interacting */}
						<div className="absolute top-1/3 left-1/4 translate-x-12 translate-y-12">
							<MousePointer2 className="w-4 h-4 text-white fill-black" />
							<div className="bg-[#FF4500] text-black text-[10px] px-1 rounded ml-2">Sarah</div>
						</div>
					</div>
				</motion.div>
			</div>
		</div>
	</section>
);

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
				Integrates with
			</motion.h2>
			<div className="flex flex-wrap justify-center gap-4 mt-10">
				{["Y.js", "Automerge", "Prosemirror", "Tldraw", "Excalidraw", "Liveblocks Client"].map((tech, i) => (
					<motion.div
						key={tech}
						initial={{ opacity: 0, y: 20 }}
						whileInView={{ opacity: 1, y: 0 }}
						viewport={{ once: true }}
						transition={{ duration: 0.5, delay: i * 0.05 }}
						className="px-2 py-1 rounded-md border border-white/5 text-zinc-400 text-xs font-mono hover:text-white hover:border-white/20 transition-colors cursor-default"
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
		<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-[#FF4500]/30 selection:text-orange-200">
			<main>
				<Hero />
				<RoomArchitecture />
				<CollaborationFeatures />
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
							Ready to go multiplayer?
						</motion.h2>
						<motion.p
							initial={{ opacity: 0, y: 20 }}
							whileInView={{ opacity: 1, y: 0 }}
							viewport={{ once: true }}
							transition={{ duration: 0.5, delay: 0.1 }}
							className="text-base text-zinc-500 mb-10 leading-relaxed"
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
							<a href="/docs" className="inline-flex items-center justify-center whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200">
								Start for Free
							</a>
							<a href="/docs/actors" className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white">
								Read the Docs
							</a>
						</motion.div>
					</div>
				</section>
			</main>
		</div>
	);
}

