"use client";

import { Terminal, ArrowRight, Check, Copy } from "lucide-react";
import { useState } from "react";
import { motion } from "framer-motion";

const CodeBlock = ({ code, fileName = "actor.ts" }) => {
	const [copied, setCopied] = useState(false);

	const handleCopy = () => {
		navigator.clipboard.writeText(code).catch(() => {});
		setCopied(true);
		setTimeout(() => setCopied(false), 2000);
	};

	return (
		<div className="relative group rounded-xl overflow-hidden border border-white/10 bg-zinc-900/50 backdrop-blur-xl shadow-2xl">
			<div className="flex items-center justify-between px-4 py-3 border-b border-white/5 bg-white/5">
				<div className="flex items-center gap-2">
					<div className="w-3 h-3 rounded-full bg-red-500/20 border border-red-500/50" />
					<div className="w-3 h-3 rounded-full bg-yellow-500/20 border border-yellow-500/50" />
					<div className="w-3 h-3 rounded-full bg-green-500/20 border border-green-500/50" />
				</div>
				<div className="text-xs text-zinc-500 font-mono">{fileName}</div>
				<button
					onClick={handleCopy}
					className="text-zinc-500 hover:text-white transition-colors"
				>
					{copied ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
				</button>
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
										// Note: this is still basic but better than before
										const parts = current.split(/([a-zA-Z0-9_$]+|"[^"]*"|'[^']*'|\s+|[(){},.;:[\]])/g).filter(Boolean);
										
										parts.forEach((part, j) => {
											const trimmed = part.trim();
											
											// Keywords
											if (["import", "from", "export", "const", "return", "async", "await", "function"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-purple-400">{part}</span>);
											}
											// Functions & Special Rivet Terms
											else if (["actor", "broadcast"].includes(trimmed)) {
												tokens.push(<span key={j} className="text-blue-400">{part}</span>);
											}
											// Object Keys / Properties / Methods
											else if (["state", "actions", "increment", "count", "push", "now", "Date"].includes(trimmed)) {
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

export const RedesignedHero = () => (
	<section className="relative pt-32 pb-20 md:pt-48 md:pb-32 overflow-hidden">
		<div className="absolute top-0 left-1/2 -translate-x-1/2 w-[1000px] h-[500px] bg-white/[0.02] blur-[100px] rounded-full pointer-events-none" />

		<div className="max-w-7xl mx-auto px-6 relative z-10">
			<div className="flex flex-col lg:flex-row gap-16 items-center">
				<div className="flex-1 max-w-2xl">
					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5 }}
						className="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-white/10 bg-white/5 text-xs font-medium text-zinc-400 mb-8 hover:border-white/20 transition-colors cursor-default"
					>
						<span className="w-2 h-2 rounded-full bg-[#FF4500] animate-pulse" />
						Rivet 2.0 is now available
						<ArrowRight className="w-3 h-3 ml-1" />
					</motion.div>

					<motion.h1
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.1 }}
						className="text-5xl md:text-7xl font-medium text-white tracking-tighter leading-[1.1] mb-6"
					>
						Stateful Backends. <br />
						<span className="text-transparent bg-clip-text bg-gradient-to-b from-zinc-200 to-zinc-500">
							Finally Solved.
						</span>
					</motion.h1>

					<motion.p
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.2 }}
						className="text-lg md:text-xl text-zinc-400 leading-relaxed mb-8 max-w-lg"
					>
						Stop faking state with databases and message queues. Rivet turns your TypeScript code into
						durable, distributed actors. No complex infrastructure, no database queries—just state that
						persists.
					</motion.p>

					<motion.div
						initial={{ opacity: 0, y: 20 }}
						animate={{ opacity: 1, y: 0 }}
						transition={{ duration: 0.5, delay: 0.3 }}
						className="flex flex-col sm:flex-row items-center gap-4"
					>
						<button className="w-full sm:w-auto h-12 px-8 rounded-full bg-white text-black font-semibold hover:bg-zinc-200 transition-colors flex items-center justify-center gap-2">
							Start Building
							<ArrowRight className="w-4 h-4" />
						</button>
						<button className="w-full sm:w-auto h-12 px-8 rounded-full border border-zinc-800 text-zinc-300 font-medium hover:text-white hover:border-zinc-600 transition-colors flex items-center justify-center gap-2 bg-black">
							<Terminal className="w-4 h-4" />
							npm install rivetkit
						</button>
					</motion.div>
				</div>

				<div className="flex-1 w-full max-w-xl">
					<motion.div
						initial={{ opacity: 0, scale: 0.95 }}
						animate={{ opacity: 1, scale: 1 }}
						transition={{ duration: 0.7, delay: 0.4, ease: [0.21, 0.47, 0.32, 0.98] }}
						className="relative"
					>
						<div className="absolute -inset-1 bg-gradient-to-r from-zinc-700 to-zinc-800 rounded-xl blur opacity-20" />
						<CodeBlock
							code={`import { actor } from "rivetkit";

// Define a robust, stateful actor in seconds

export const counter = actor({
  // State is type-safe and persistent
  state: { count: 0 },

  actions: {
    increment: (c) => {
      c.state.count++;
      
      // Realtime by default
      c.broadcast("updated", c.state);
      
      return c.state.count;
    }
  }
});`}
						/>
					</motion.div>
				</div>
			</div>
		</div>
	</section>
);
