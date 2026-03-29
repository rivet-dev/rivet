"use client";

import { useCallback, useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Icon } from "@rivet-gg/icons";
import type { RegistryEntry } from "@/data/registry";

const CATEGORY_ORDER: { type: string; label: string; description: string }[] = [
	{
		type: "agent",
		label: "Agents",
		description:
			"Coding agents with programmatic API access and universal transcript format (ACP).",
	},
	{
		type: "file-system",
		label: "File Systems",
		description:
			"Mount these file systems as the root or at any sub-path inside the VM.",
	},
	{
		type: "sandbox-extension",
		label: "Sandbox Extensions",
		description:
			"agentOS is a hybrid OS. Mount sandbox file systems and interact with them via tools for heavier workloads. Use agentOS natively for lightweight tasks.",
	},
	{
		type: "tool",
		label: "Tools",
		description:
			"Host-side tools and integrations that extend agent capabilities.",
	},
	{
		type: "software",
		label: "Software",
		description:
			"WASM command packages that run inside the VM. Install individually or use meta-packages.",
	},
];

const MAX_GRID_ITEMS = 9;
const CAROUSEL_INTERVAL = 5000;

type RegistryTheme = "dark" | "light";

function entryHref(hrefBase: string, slug: string) {
	return `${hrefBase}/${slug}`;
}

function EntryIcon({
	entry,
	size = 24,
	theme,
}: {
	entry: RegistryEntry;
	size?: number;
	theme: RegistryTheme;
}) {
	if (entry.image) {
		return (
			<img
				src={entry.image}
				alt={entry.title}
				width={size}
				height={size}
				className="object-contain"
			/>
		);
	}

	if (entry.icon) {
		return (
			<Icon
				icon={entry.icon}
				style={{ width: size, height: size }}
				className={theme === "light" ? "text-zinc-900" : "text-white"}
			/>
		);
	}

	return (
		<span
			className={
				theme === "light"
					? "select-none font-mono font-bold text-zinc-600"
					: "select-none font-mono font-bold text-white/80"
			}
			style={{ fontSize: size * 0.6, lineHeight: 1 }}
		>
			{entry.title.charAt(0).toUpperCase()}
		</span>
	);
}

function MonoIcon({
	entry,
	size = 24,
	theme,
}: {
	entry: RegistryEntry;
	size?: number;
	theme: RegistryTheme;
}) {
	if (entry.image) {
		return (
			<img
				src={entry.image}
				alt={entry.title}
				width={size}
				height={size}
				className="object-contain"
				style={
					theme === "light"
						? { filter: "brightness(0)", opacity: 0.4 }
						: { filter: "brightness(0) invert(1)", opacity: 0.6 }
				}
			/>
		);
	}

	if (entry.icon) {
		return (
			<Icon
				icon={entry.icon}
				style={{ width: size, height: size }}
				className={theme === "light" ? "text-zinc-400" : "text-white/60"}
			/>
		);
	}

	return (
		<span
			className={
				theme === "light"
					? "select-none font-mono font-bold text-zinc-300"
					: "select-none font-mono font-bold text-white/40"
			}
			style={{ fontSize: size * 0.6, lineHeight: 1 }}
		>
			{entry.title.charAt(0).toUpperCase()}
		</span>
	);
}

function FeaturedCarousel({
	entries,
	theme,
	hrefBase,
}: {
	entries: RegistryEntry[];
	theme: RegistryTheme;
	hrefBase: string;
}) {
	const [index, setIndex] = useState(0);
	const [direction, setDirection] = useState(1);

	const go = useCallback(
		(next: number) => {
			if (next === index) return;
			setDirection(next > index ? 1 : -1);
			setIndex(next);
		},
		[index],
	);

	const goNext = useCallback(() => {
		setDirection(1);
		setIndex((current) => (current + 1) % entries.length);
	}, [entries.length]);

	useEffect(() => {
		const timer = setInterval(goNext, CAROUSEL_INTERVAL);
		return () => clearInterval(timer);
	}, [goNext, index]);

	const entry = entries[index];
	const light = theme === "light";

	const variants = {
		enter: (dir: number) => ({ x: dir > 0 ? 300 : -300, opacity: 0 }),
		center: { x: 0, opacity: 1 },
		exit: (dir: number) => ({ x: dir > 0 ? -300 : 300, opacity: 0 }),
	};

	return (
		<div className="mb-14">
			<div
				className={
					light
						? "relative h-[320px] overflow-hidden rounded-2xl border border-zinc-200 bg-gradient-to-br from-zinc-50 to-white"
						: "relative h-[320px] overflow-hidden rounded-2xl border border-white/15 bg-gradient-to-br from-white/8 to-white/2"
				}
			>
				<AnimatePresence mode="wait" custom={direction}>
					<motion.a
						key={entry.slug}
						href={entryHref(hrefBase, entry.slug)}
						custom={direction}
						variants={variants}
						initial="enter"
						animate="center"
						exit="exit"
						transition={{ duration: 0.3, ease: "easeInOut" }}
						className="absolute inset-0 flex flex-col items-center justify-center px-10 text-center no-underline"
					>
						<div className="mb-6 flex h-20 w-20 items-center justify-center">
							<EntryIcon entry={entry} size={64} theme={theme} />
						</div>
						<h3
							className={
								light
									? "mb-3 text-2xl font-semibold text-zinc-900"
									: "mb-3 text-2xl font-semibold text-white"
							}
						>
							{entry.title}
						</h3>
						<p
							className={
								light
									? "max-w-md line-clamp-3 text-base leading-relaxed text-zinc-500"
									: "max-w-md line-clamp-3 text-base leading-relaxed text-white/60"
							}
						>
							{entry.description}
						</p>
					</motion.a>
				</AnimatePresence>
			</div>

			<div className="mt-3 flex justify-center gap-1">
				{entries.map((candidate, candidateIndex) => (
					<button
						key={candidate.slug}
						onClick={() => go(candidateIndex)}
						className={`flex items-center gap-2 rounded-lg px-3 py-2 transition-opacity duration-200 ${
							candidateIndex === index
								? "opacity-100"
								: "opacity-30 hover:opacity-50"
						}`}
					>
						<div className="flex h-5 w-5 shrink-0 items-center justify-center">
							<MonoIcon entry={candidate} size={16} theme={theme} />
						</div>
						<span
							className={
								light
									? "text-xs font-medium text-zinc-900"
									: "text-xs font-medium text-white"
							}
						>
							{candidate.title}
						</span>
					</button>
				))}
			</div>
		</div>
	);
}

function CategorySection({
	label,
	description,
	entries,
	theme,
	hrefBase,
}: {
	label: string;
	description: string;
	entries: RegistryEntry[];
	theme: RegistryTheme;
	hrefBase: string;
}) {
	const [expanded, setExpanded] = useState(false);
	const needsShowAll = entries.length > MAX_GRID_ITEMS;
	const visible = expanded
		? entries
		: entries.slice(0, MAX_GRID_ITEMS - (needsShowAll ? 1 : 0));
	const light = theme === "light";

	return (
		<section className="mb-12">
			<h2
				className={
					light
						? "mb-2 text-2xl font-semibold text-zinc-900"
						: "mb-2 text-2xl font-semibold text-white"
				}
			>
				{label}
			</h2>
			<p
				className={
					light ? "mb-4 text-sm text-zinc-500" : "mb-4 text-sm text-white/50"
				}
			>
				{description}
			</p>
			<div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
				{visible.map((entry) => (
					<a
						key={entry.slug}
						href={entryHref(hrefBase, entry.slug)}
						className={`group flex h-36 flex-col rounded-xl border p-5 text-left no-underline transition-all duration-200 ${
							entry.status === "coming-soon" ? "opacity-60" : ""
						} ${
							light
								? "border-zinc-200 bg-white hover:border-zinc-300 hover:shadow-sm"
								: "border-white/10 bg-white/2 hover:border-white/25"
						}`}
					>
						<div className="mb-2 flex items-center gap-2.5">
							<div className="flex h-6 w-6 shrink-0 items-center justify-center">
								<EntryIcon entry={entry} size={20} theme={theme} />
							</div>
							<h3
								className={
									light
										? "truncate font-semibold text-zinc-900"
										: "truncate font-semibold text-white"
								}
							>
								{entry.title}
							</h3>
							{entry.status === "coming-soon" && (
								<span
									className={
										light
											? "shrink-0 rounded-full border border-zinc-200 px-1.5 py-0.5 text-[10px] text-zinc-400"
											: "shrink-0 rounded-full border border-white/20 px-1.5 py-0.5 text-[10px] text-white/50"
									}
								>
									Coming Soon
								</span>
							)}
						</div>
						<p
							className={
								light
									? "line-clamp-3 text-sm leading-relaxed text-zinc-500"
									: "line-clamp-3 text-sm leading-relaxed text-white/60"
							}
						>
							{entry.description}
						</p>
					</a>
				))}

				{needsShowAll && !expanded && (
					<button
						onClick={() => setExpanded(true)}
						className={
							light
								? "flex h-36 flex-col items-center justify-center gap-2 rounded-xl border border-zinc-200 bg-zinc-50 p-5 transition-all duration-200 hover:border-zinc-300"
								: "flex h-36 flex-col items-center justify-center gap-2 rounded-xl border border-white/10 bg-white/2 p-5 transition-all duration-200 hover:border-white/25"
						}
					>
						<span
							className={
								light
									? "text-2xl font-light text-zinc-300"
									: "text-2xl font-light text-white/30"
							}
						>
							+{entries.length - (MAX_GRID_ITEMS - 1)}
						</span>
						<span
							className={
								light
									? "text-sm font-medium text-zinc-500"
									: "text-sm font-medium text-white/60"
							}
						>
							Show all ({entries.length})
						</span>
					</button>
				)}
			</div>
		</section>
	);
}

export default function RegistryPageClient({
	entries,
	theme = "dark",
	hrefBase = "/agent-os/registry",
}: {
	entries: RegistryEntry[];
	theme?: RegistryTheme;
	hrefBase?: string;
}) {
	const featured = entries.filter((entry) => entry.featured);
	const light = theme === "light";

	const categories = CATEGORY_ORDER.map(({ type, label, description }) => ({
		label,
		description,
		entries: entries.filter((entry) =>
			entry.types.includes(type as RegistryEntry["types"][number]),
		),
	})).filter((category) => category.entries.length > 0);

	return (
		<>
			{featured.length > 0 && (
				<FeaturedCarousel
					entries={featured}
					theme={theme}
					hrefBase={hrefBase}
				/>
			)}

			{categories.map((category) => (
				<CategorySection
					key={category.label}
					label={category.label}
					description={category.description}
					entries={category.entries}
					theme={theme}
					hrefBase={hrefBase}
				/>
			))}

			<div
				className={
					light
						? "mt-8 border-t border-zinc-200 pt-8 text-center"
						: "mt-8 border-t border-white/10 pt-8 text-center"
				}
			>
				<p
					className={
						light ? "mb-4 text-sm text-zinc-500" : "mb-4 text-sm text-white/50"
					}
				>
					Want to add your own package to the registry?
				</p>
				<div className="flex items-center justify-center gap-3">
					<a
						href="https://github.com/rivet-dev/agent-os-registry/blob/main/CONTRIBUTING.md"
						target="_blank"
						rel="noopener noreferrer"
						className={
							light
								? "inline-flex items-center gap-2 rounded-lg border border-zinc-200 px-5 py-2.5 text-sm font-medium text-zinc-600 transition-all duration-200 no-underline hover:border-zinc-300 hover:text-zinc-900"
								: "inline-flex items-center gap-2 rounded-lg border border-white/20 px-5 py-2.5 text-sm font-medium text-white/80 transition-all duration-200 no-underline hover:border-white/40 hover:text-white"
						}
					>
						Publish a Package
						<svg
							xmlns="http://www.w3.org/2000/svg"
							width={14}
							height={14}
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							strokeWidth={2}
							strokeLinecap="round"
							strokeLinejoin="round"
						>
							<path d="M7 7h10v10" />
							<path d="M7 17 17 7" />
						</svg>
					</a>
					<a
						href="https://github.com/rivet-dev/agent-os-registry/issues"
						target="_blank"
						rel="noopener noreferrer"
						className={
							light
								? "inline-flex items-center gap-2 rounded-lg border border-zinc-200 px-5 py-2.5 text-sm font-medium text-zinc-600 transition-all duration-200 no-underline hover:border-zinc-300 hover:text-zinc-900"
								: "inline-flex items-center gap-2 rounded-lg border border-white/20 px-5 py-2.5 text-sm font-medium text-white/80 transition-all duration-200 no-underline hover:border-white/40 hover:text-white"
						}
					>
						Request an Extension
						<svg
							xmlns="http://www.w3.org/2000/svg"
							width={14}
							height={14}
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							strokeWidth={2}
							strokeLinecap="round"
							strokeLinejoin="round"
						>
							<path d="M7 7h10v10" />
							<path d="M7 17 17 7" />
						</svg>
					</a>
				</div>
			</div>
		</>
	);
}
