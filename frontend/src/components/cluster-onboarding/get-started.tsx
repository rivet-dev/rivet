import { faArrowUpRight, faCheck, faCopy, Icon } from "@rivet-gg/icons";
import { type CSSProperties, useState } from "react";
import { CopyTrigger } from "@/components/copy-area";
import { cn } from "@/components/lib/utils";
import { publicUrl } from "@/lib/utils";
import {
	AGENTOS_CONTENT,
	DEPLOY_ACTOR_CONTENT,
	DURABLE_STREAMS_CONTENT,
	DYNAMIC_APPS_CONTENT,
	type ProductPageContent,
	productAgentPrompt,
	WORKFLOWS_CONTENT,
} from "./product-content";

/**
 * What to do with a fresh cluster, styled like the product cards on the
 * rivet.dev home page ("Whatever the workload, there's an Actor for it"):
 * a solid product colour, a faint line-art motif that animates on hover,
 * and the white product glyph with a one-line pitch. Clicking a card opens
 * the product's quickstart in the sidebar; the pill copies the agent prompt.
 */
type Motif = "squircles" | "rails" | "circles" | "cells" | "streams";

const GET_STARTED: {
	product: ProductPageContent;
	/** Card title; defaults to the product's name. */
	title?: string;
	tagline: string;
	/** White glyph shown next to the title (the mark without its tile). */
	logoFileName: string;
	/** Card colour, from the site's `bg-product-*` palette. */
	color: string;
	motif: Motif;
	badge?: string;
}[] = [
	{
		product: AGENTOS_CONTENT,
		tagline: "A filesystem, shell, and network for code you did not write",
		logoFileName: "agentos-logo.svg",
		color: "rgb(156 58 63)",
		motif: "circles",
	},
	{
		product: DYNAMIC_APPS_CONTENT,
		tagline: "A backend per user, deployed the moment it is generated",
		logoFileName: "dynamic-apps-logo.svg",
		color: "rgb(47 107 75)",
		motif: "cells",
		badge: "Preview",
	},
	{
		product: WORKFLOWS_CONTENT,
		tagline: "Multi-step operations that replay instead of starting over",
		logoFileName: "workflows-logo.svg",
		color: "rgb(106 76 147)",
		motif: "rails",
	},
	{
		product: DURABLE_STREAMS_CONTENT,
		tagline: "Real-time streams with replayable history",
		logoFileName: "durable-streams-mark.svg",
		color: "rgb(44 90 122)",
		motif: "streams",
	},
	{
		product: DEPLOY_ACTOR_CONTENT,
		title: "Empty Actor",
		tagline:
			"Long-lived processes with state, realtime, and scheduling built in",
		logoFileName: "actors-logo.svg",
		color: "rgb(91 93 99)",
		motif: "squircles",
	},
];

const REGISTRY_URL = "https://rivet.dev/registry/";

export function GetStartedCards({
	onOpen,
}: {
	/** Opens the product's page in the sidebar. */
	onOpen: (product: ProductPageContent) => void;
}) {
	return (
		<>
			<style>{MOTIF_STYLES}</style>
			<div className="grid grid-cols-[repeat(auto-fill,minmax(15rem,1fr))] gap-4 xl:grid-cols-3">
				{GET_STARTED.map((entry) => (
					<GetStartedCard
						key={entry.product.id}
						{...entry}
						onOpen={() => onOpen(entry.product)}
					/>
				))}
				<a
					href={REGISTRY_URL}
					target="_blank"
					rel="noreferrer"
					className="flex min-h-[168px] flex-col items-center justify-center gap-3 rounded-xl border bg-muted/40 p-5 text-center text-sm font-medium transition-colors hover:border-foreground/25 hover:bg-muted/70"
				>
					<span className="flex items-center gap-1.5">
						Explore more Actors
						<Icon
							icon={faArrowUpRight}
							className="text-xs text-muted-foreground"
						/>
					</span>
				</a>
			</div>
		</>
	);
}

function GetStartedCard({
	product,
	title,
	tagline,
	logoFileName,
	color,
	motif,
	badge,
	onOpen,
}: (typeof GET_STARTED)[number] & { onOpen: () => void }) {
	const [copied, setCopied] = useState(false);
	const name = title ?? product.label;
	return (
		<div
			className="gs-card group relative flex min-h-[200px] flex-col justify-between gap-8 overflow-hidden rounded-xl p-5 text-white transition-shadow focus-within:ring-2 focus-within:ring-white/70 focus-within:ring-offset-2 focus-within:ring-offset-background hover:shadow-lg"
			style={{ backgroundColor: color }}
		>
			<button
				type="button"
				aria-label={`${name} quickstart`}
				className="absolute inset-0 focus-visible:outline-none"
				onClick={onOpen}
			/>
			<div
				aria-hidden="true"
				className="pointer-events-none absolute inset-0 overflow-hidden"
				style={{
					maskImage:
						"linear-gradient(to bottom, black 55%, transparent 84%)",
					WebkitMaskImage:
						"linear-gradient(to bottom, black 55%, transparent 84%)",
				}}
			>
				<div className="absolute left-0 top-1/2 aspect-[4/5] w-full -translate-y-1/2">
					<MotifArt motif={motif} />
				</div>
			</div>
			<span
				aria-hidden="true"
				className="pointer-events-none absolute inset-0 bg-gradient-to-br from-white/[0.09] via-transparent to-black/[0.12]"
			/>
			<div className="pointer-events-none relative flex items-start justify-between gap-2">
				<span>
					{badge ? (
						<span className="whitespace-nowrap rounded border border-white/20 px-1.5 py-px text-[11px] font-medium leading-4 text-white/70">
							{badge}
						</span>
					) : null}
				</span>
				<CopyTrigger
					value={productAgentPrompt(product)}
					onClick={() => setCopied(true)}
				>
					<button
						type="button"
						className="pointer-events-auto flex items-center gap-1.5 whitespace-nowrap rounded border border-white/25 bg-black/10 px-2 py-0.5 text-[11px] font-medium leading-4 text-white/85 backdrop-blur-sm transition-colors hover:bg-white/15 hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white/70"
					>
						<Icon
							icon={copied ? faCheck : faCopy}
							className="text-[10px]"
						/>
						{copied ? "Copied" : "Copy agent prompt"}
					</button>
				</CopyTrigger>
			</div>
			<div className="pointer-events-none relative">
				<div className="flex items-center gap-2.5">
					<span
						aria-hidden="true"
						className="inline-block h-6 w-6 shrink-0 bg-white"
						style={maskStyle(logoFileName)}
					/>
					<div className="text-lg font-medium leading-snug tracking-[-0.01em]">
						{name}
					</div>
				</div>
				<p className="mt-2 text-balance text-[15px] font-medium leading-relaxed text-white/90">
					{tagline}
				</p>
			</div>
		</div>
	);
}

function maskStyle(fileName: string): CSSProperties {
	const url = `url(${publicUrl(`images/brand/${fileName}`)})`;
	return {
		maskImage: url,
		WebkitMaskImage: url,
		maskRepeat: "no-repeat",
		WebkitMaskRepeat: "no-repeat",
		maskPosition: "center",
		WebkitMaskPosition: "center",
		maskSize: "contain",
		WebkitMaskSize: "contain",
	};
}

// ---------------------------------------------------------------------------
// Motifs, ported from the rivet.dev home cards. Each draws into a 320x400
// box stretched to the 4:5 container; stroke widths are in screen pixels.

const LINE_SOFT = "rgba(244, 241, 231, 0.2)";
const LINE_STRONG = "rgba(244, 241, 231, 0.34)";
const NODE_STROKE = "rgba(244, 241, 231, 0.22)";

function MotifArt({ motif }: { motif: Motif }) {
	switch (motif) {
		case "squircles":
			return <SquirclesMotif />;
		case "rails":
			return <RailsMotif />;
		case "circles":
			return <CirclesMotif />;
		case "cells":
			return <CellsMotif />;
		case "streams":
			return <StreamsMotif />;
	}
}

/** Actors: a field of tilted logo outlines that drifts upward on hover. */
const SQUIRCLES: [left: number, top: number, tilt: number, opacity: number][] =
	[
		[-19, -13.5, -5, 0.18],
		[29, -11.5, 3, 0.24],
		[75, -13, -2, 0.18],
		[4, 13.5, 5, 0.2],
		[52, 11.5, -4, 0.26],
		[-17, 36.5, 4, 0.17],
		[30, 38.5, -5, 0.22],
		[76, 36.5, 3, 0.18],
		[6, 63.5, -3, 0.2],
		[50, 61, 5, 0.24],
	];

function SquirclesMotif() {
	return (
		<div className="gs-track absolute inset-x-0 top-0 h-[240%]">
			{[0, 50].map((fieldTop) => (
				<div
					key={fieldTop}
					className="absolute left-0 h-1/2 w-full"
					style={{ top: `${fieldTop}%` }}
				>
					{SQUIRCLES.map(([left, top, tilt, opacity], i) => (
						<span
							// biome-ignore lint/suspicious/noArrayIndexKey: static list
							key={i}
							className="absolute aspect-square w-[44%]"
							style={{
								left: `${left}%`,
								top: `${top}%`,
								transform: `rotate(${tilt}deg)`,
								opacity,
							}}
						>
							<svg
								aria-hidden="true"
								className="block h-full w-full"
								viewBox="0 0 128 128"
							>
								<rect
									x="18.25"
									y="18.25"
									width="91.5"
									height="91.5"
									rx="25.75"
									fill="none"
									stroke="#fff"
									strokeWidth="8.5"
								/>
							</svg>
						</span>
					))}
				</div>
			))}
		</div>
	);
}

/** Workflows: three looping rails with a bright pulse that runs them on hover. */
const RAILS: { d: string; bright: boolean }[] = [
	{
		d: "M -180 34 H 262 a 52 52 0 0 1 0 104 H 58 a 24 24 0 0 0 0 48 H 262 a 52 52 0 0 1 0 104 H -180",
		bright: false,
	},
	{
		d: "M -180 48 H 262 a 38 38 0 0 1 0 76 H 58 a 38 38 0 0 0 0 76 H 262 a 38 38 0 0 1 0 76 H -180",
		bright: true,
	},
	{
		d: "M -180 62 H 262 a 24 24 0 0 1 0 48 H 58 a 52 52 0 0 0 0 104 H 262 a 24 24 0 0 1 0 48 H -180",
		bright: false,
	},
];

function RailsMotif() {
	return (
		<svg
			aria-hidden="true"
			className="h-full w-full"
			viewBox="0 0 320 400"
			preserveAspectRatio="none"
		>
			{RAILS.map(({ d, bright }, i) => (
				<g
					// biome-ignore lint/suspicious/noArrayIndexKey: static list
					key={i}
					style={{ "--lane-delay": `${i * 140}ms` } as CSSProperties}
				>
					<path
						d={d}
						pathLength={1}
						vectorEffect="non-scaling-stroke"
						fill="none"
						stroke={bright ? LINE_STRONG : LINE_SOFT}
						strokeWidth={1}
					/>
					<path
						className="gs-pulse"
						d={d}
						pathLength={1}
						vectorEffect="non-scaling-stroke"
						fill="none"
						stroke="rgba(244, 241, 231, 0.88)"
						strokeWidth={1.5}
						strokeLinecap="round"
						strokeDasharray="0.1 0.9"
						strokeDashoffset={1}
					/>
				</g>
			))}
		</svg>
	);
}

/** agentOS: overlapping circle outlines that slide past each other on hover. */
const CIRCLES: [
	cx: number,
	cy: number,
	forward: boolean,
	bright: boolean,
	delay: number,
][] = [
	[16, 36, true, true, 0],
	[96, 108, false, false, 0],
	[176, 36, true, false, 120],
	[256, 108, false, true, 120],
	[16, 180, true, false, 180],
	[96, 252, false, true, 180],
	[176, 180, true, true, 300],
	[256, 252, false, false, 300],
];

function CirclesMotif() {
	return (
		<svg
			aria-hidden="true"
			className="h-full w-full"
			viewBox="0 0 320 400"
			preserveAspectRatio="none"
		>
			{CIRCLES.map(([cx, cy, forward, bright, delay], i) => (
				<circle
					// biome-ignore lint/suspicious/noArrayIndexKey: static list
					key={i}
					className={cn(
						"gs-circle",
						forward ? "gs-circle-fwd" : "gs-circle-back",
					)}
					style={{ "--circle-delay": `${delay}ms` } as CSSProperties}
					cx={cx}
					cy={cy}
					r={62}
					vectorEffect="non-scaling-stroke"
					fill="none"
					strokeWidth={1}
					stroke={bright ? LINE_STRONG : LINE_SOFT}
				/>
			))}
		</svg>
	);
}

/** Dynamic Apps: a Game-of-Life field; still cells, blinkers, and a glider. */
const CELLS: {
	x: number;
	y: number;
	kind: "on" | "a" | "b" | "still";
	soft?: boolean;
}[] = [
	{ x: 100, y: 68, kind: "on", soft: true },
	{ x: 4, y: 100, kind: "on", soft: true },
	{ x: 36, y: 68, kind: "a", soft: true },
	{ x: 68, y: 68, kind: "a", soft: true },
	{ x: 36, y: 100, kind: "a", soft: true },
	{ x: 68, y: 100, kind: "a", soft: true },
	{ x: 68, y: 36, kind: "b", soft: true },
	{ x: 4, y: 68, kind: "b", soft: true },
	{ x: 100, y: 100, kind: "b", soft: true },
	{ x: 36, y: 132, kind: "b", soft: true },
	{ x: 196, y: 4, kind: "on", soft: true },
	{ x: 228, y: 4, kind: "on", soft: true },
	{ x: 196, y: 36, kind: "on", soft: true },
	{ x: 292, y: 68, kind: "on", soft: true },
	{ x: 260, y: 100, kind: "on", soft: true },
	{ x: 292, y: 100, kind: "on", soft: true },
	{ x: 228, y: 36, kind: "a" },
	{ x: 260, y: 68, kind: "a" },
	{ x: 4, y: 228, kind: "still" },
	{ x: 36, y: 228, kind: "still" },
	{ x: 4, y: 260, kind: "still" },
	{ x: 36, y: 260, kind: "still" },
];
const GLIDER: { x: number; y: number }[] = [
	{ x: 196, y: 228 },
	{ x: 228, y: 260 },
	{ x: 164, y: 292 },
	{ x: 196, y: 292 },
	{ x: 228, y: 292 },
];

function Cell({
	x,
	y,
	kind,
	soft,
}: {
	x: number;
	y: number;
	kind: "on" | "a" | "b" | "still";
	soft?: boolean;
}) {
	const still = kind === "still";
	return (
		<rect
			className={cn(
				kind === "a" && "gs-cell-a",
				kind === "b" && "gs-cell-b",
			)}
			x={x}
			y={y}
			width={24}
			height={24}
			rx={4}
			vectorEffect="non-scaling-stroke"
			fill={still ? "none" : soft ? LINE_SOFT : LINE_STRONG}
			stroke={still ? NODE_STROKE : undefined}
			strokeWidth={still ? 1 : undefined}
		/>
	);
}

function CellsMotif() {
	return (
		<svg
			aria-hidden="true"
			className="h-full w-full"
			viewBox="0 0 320 400"
			preserveAspectRatio="none"
		>
			{CELLS.map((c) => (
				<Cell key={`${c.x}-${c.y}`} {...c} />
			))}
			<g className="gs-glider">
				{GLIDER.map((c) => (
					<Cell key={`${c.x}-${c.y}`} {...c} kind="on" />
				))}
			</g>
		</svg>
	);
}

/** Durable Streams: event lanes with records that advance on hover. */
const LANES = [48, 96, 144, 192];

function StreamsMotif() {
	return (
		<svg
			aria-hidden="true"
			className="h-full w-full"
			viewBox="0 0 320 400"
			preserveAspectRatio="none"
		>
			{LANES.map((y, i) => (
				<g
					key={y}
					style={{ "--lane-delay": `${i * -0.9}s` } as CSSProperties}
				>
					<line
						x1={-20}
						x2={340}
						y1={y}
						y2={y}
						vectorEffect="non-scaling-stroke"
						stroke={i % 2 ? LINE_SOFT : LINE_STRONG}
						strokeWidth={1}
					/>
					<line
						className="gs-stream"
						x1={-20}
						x2={340}
						y1={y}
						y2={y}
						pathLength={1}
						vectorEffect="non-scaling-stroke"
						stroke="rgba(244, 241, 231, 0.7)"
						strokeWidth={5}
						strokeLinecap="round"
						strokeDasharray="0.04 0.21"
					/>
				</g>
			))}
		</svg>
	);
}

const MOTIF_STYLES = `
.gs-card .gs-track { animation: gs-rise 18s linear infinite paused; will-change: transform; }
.gs-card:hover .gs-track { animation-play-state: running; }
@keyframes gs-rise { to { transform: translate3d(0, -50%, 0); } }

.gs-card .gs-pulse { animation: gs-pulse-travel 3.6s cubic-bezier(0.3, 0.2, 0.7, 0.9) infinite paused; animation-delay: var(--lane-delay); }
.gs-card:hover .gs-pulse { animation-play-state: running; }
@keyframes gs-pulse-travel { from { stroke-dashoffset: 1; } to { stroke-dashoffset: 0.1; } }

.gs-card .gs-circle { transform-box: view-box; transform-origin: center; animation: 3.2s cubic-bezier(0.22, 1, 0.36, 1) infinite paused; animation-delay: var(--circle-delay); }
.gs-card .gs-circle-fwd { animation-name: gs-circle-fwd; }
.gs-card .gs-circle-back { animation-name: gs-circle-back; }
.gs-card:hover .gs-circle { animation-play-state: running; }
@keyframes gs-circle-fwd { 0%, 7%, 93%, 100% { transform: translate(0, 0); } 43%, 57% { transform: translate(25%, 18%); } }
@keyframes gs-circle-back { 0%, 7%, 93%, 100% { transform: translate(0, 0); } 43%, 57% { transform: translate(-25%, -18%); } }

.gs-card .gs-cell-a, .gs-card .gs-cell-b { animation: gs-tick 2s steps(1) infinite paused; }
.gs-card .gs-cell-b { opacity: 0; animation-delay: -1s; }
.gs-card .gs-glider { transform-box: view-box; animation: gs-walk 16s linear infinite paused; }
.gs-card:hover .gs-cell-a, .gs-card:hover .gs-cell-b, .gs-card:hover .gs-glider { animation-play-state: running; }
@keyframes gs-tick { 0% { opacity: 1; } 50% { opacity: 0; } 100% { opacity: 1; } }
@keyframes gs-walk { from { transform: translate(0, 0); } to { transform: translate(40%, 32%); } }

.gs-card .gs-stream { animation: gs-stream-advance 3s linear infinite paused; animation-delay: var(--lane-delay); }
.gs-card:hover .gs-stream { animation-play-state: running; }
@keyframes gs-stream-advance { from { stroke-dashoffset: 0; } to { stroke-dashoffset: -0.25; } }

@media (prefers-reduced-motion: reduce) {
	.gs-card .gs-track, .gs-card .gs-pulse, .gs-card .gs-circle, .gs-card .gs-cell-a, .gs-card .gs-cell-b, .gs-card .gs-glider, .gs-card .gs-stream { animation: none !important; }
}
`;
