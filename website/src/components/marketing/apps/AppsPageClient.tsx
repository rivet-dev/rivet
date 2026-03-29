"use client";

import { useState } from "react";

interface App {
	title: string;
	package: string;
	description: string;
	types: ("file-system" | "tool" | "agent")[];
}

const TYPE_LABELS: Record<string, string> = {
	"file-system": "File System",
	tool: "Tool",
	agent: "Agent",
};

const TYPE_COLORS: Record<string, string> = {
	"file-system": "bg-blue-500/20 text-blue-400 border-blue-500/30",
	tool: "bg-emerald-500/20 text-emerald-400 border-emerald-500/30",
	agent: "bg-orange-500/20 text-orange-400 border-orange-500/30",
};

function AppModal({
	app,
	onClose,
}: { app: App; onClose: () => void }) {
	const installCmd = `npm install ${app.package}`;
	const [copied, setCopied] = useState(false);

	function handleCopy() {
		navigator.clipboard.writeText(installCmd);
		setCopied(true);
		setTimeout(() => setCopied(false), 2000);
	}

	return (
		<div
			className="fixed inset-0 z-50 flex items-center justify-center bg-black/70"
			onClick={(e) => {
				if (e.target === e.currentTarget) onClose();
			}}
		>
			<div className="relative w-full max-w-md mx-4 bg-[#181818] rounded-2xl shadow-lg text-white p-6">
				<button
					onClick={onClose}
					className="absolute top-4 right-4 text-white/60 hover:text-white text-2xl font-bold focus:outline-none"
					aria-label="Close"
				>
					&times;
				</button>

				<h2 className="text-xl font-semibold mb-2">{app.title}</h2>

				<div className="flex gap-2 mb-4">
					{app.types.map((type) => (
						<span
							key={type}
							className={`text-xs px-2 py-0.5 rounded-full border ${TYPE_COLORS[type]}`}
						>
							{TYPE_LABELS[type]}
						</span>
					))}
				</div>

				<p className="text-white/70 text-sm mb-6">{app.description}</p>

				<div className="mb-4">
					<label className="text-xs text-white/50 mb-1 block">
						Install
					</label>
					<div className="flex items-center gap-2 bg-[#111] rounded-lg border border-white/10 px-3 py-2">
						<code className="text-sm text-white/90 flex-1 font-mono">
							{installCmd}
						</code>
						<button
							onClick={handleCopy}
							className="text-xs text-white/50 hover:text-white shrink-0 transition-colors"
						>
							{copied ? "Copied" : "Copy"}
						</button>
					</div>
				</div>

				<a
					href={`https://www.npmjs.com/package/${app.package}`}
					target="_blank"
					rel="noopener noreferrer"
					className="inline-flex items-center justify-center w-full rounded-lg bg-white/10 hover:bg-white/15 border border-white/10 text-white text-sm font-medium py-2 transition-colors"
				>
					View on npm
				</a>
			</div>
		</div>
	);
}

export default function AppsPageClient({ apps }: { apps: App[] }) {
	const [selected, setSelected] = useState<App | null>(null);
	const [filter, setFilter] = useState<string | null>(null);

	const allTypes = Array.from(new Set(apps.flatMap((a) => a.types))).sort();
	const filtered = filter
		? apps.filter((a) => a.types.includes(filter as App["types"][number]))
		: apps;

	return (
		<>
			<div className="flex gap-2 mb-8 justify-center flex-wrap">
				<button
					onClick={() => setFilter(null)}
					className={`text-sm px-3 py-1 rounded-full border transition-colors ${
						filter === null
							? "bg-white/15 border-white/30 text-white"
							: "bg-white/5 border-white/10 text-white/60 hover:text-white hover:border-white/20"
					}`}
				>
					All
				</button>
				{allTypes.map((type) => (
					<button
						key={type}
						onClick={() =>
							setFilter(filter === type ? null : type)
						}
						className={`text-sm px-3 py-1 rounded-full border transition-colors ${
							filter === type
								? "bg-white/15 border-white/30 text-white"
								: "bg-white/5 border-white/10 text-white/60 hover:text-white hover:border-white/20"
						}`}
					>
						{TYPE_LABELS[type]}
					</button>
				))}
			</div>

			<div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
				{filtered.map((app) => (
					<button
						key={app.package}
						onClick={() => setSelected(app)}
						className="text-left group rounded-xl bg-white/2 border border-white/10 hover:border-white/25 p-5 transition-all duration-200"
					>
						<h3 className="font-semibold text-white mb-1">
							{app.title}
						</h3>
						<div className="flex gap-1.5 mb-2">
							{app.types.map((type) => (
								<span
									key={type}
									className={`text-[10px] px-1.5 py-0.5 rounded-full border ${TYPE_COLORS[type]}`}
								>
									{TYPE_LABELS[type]}
								</span>
							))}
						</div>
						<p className="text-white/60 text-sm leading-relaxed">
							{app.description}
						</p>
					</button>
				))}
			</div>

			{selected && (
				<AppModal
					app={selected}
					onClose={() => setSelected(null)}
				/>
			)}
		</>
	);
}
