import { faBucket, faGlobe, Icon, type IconProp } from "@rivet-gg/icons";
import confetti from "canvas-confetti";
import { type ReactNode, useRef, useState } from "react";
import { LogoMark } from "@/app/logo";
import { cn } from "@/components/lib/utils";
import { WithTooltip } from "@/components/ui/tooltip";
import { publicUrl } from "@/lib/utils";
import type { ClusterNode, NodeStatus } from "./model";
import { cubicBezier, type Pose, type SceneOptions, useScene } from "./scene";

interface DeploymentMapProps {
	nodes: ClusterNode[];
	/** Host of the external endpoint, e.g. api.test-cluster-3.rivet.run. */
	externalHost: string;
	/**
	 * Stretches every animation by this factor (0.1 = ten times slower).
	 * Only the animation lab sets it, to inspect motion frame by frame.
	 */
	timeScale?: number;
}

/**
 * How traffic flows: public internet → control plane (with storage
 * attached) → actor runners. Each group shows its nodes stacked by version,
 * and traffic pulses along the connectors.
 */
export function DeploymentMap({
	nodes,
	externalHost,
	timeScale = 1,
}: DeploymentMapProps) {
	const controlPlane = nodes.filter((n) => n.pool === undefined);
	const runners = nodes.filter((n) => n.pool !== undefined);
	const celebration = useCelebration();

	const storage = (
		<Connector horizontal flow={FLOW.storage}>
			<Tile
				mark={<Squircle icon={faBucket} className="bg-red-600" />}
				title="Storage"
				tint={FLOW.storage.color}
			/>
		</Connector>
	);

	return (
		<div
			className="relative overflow-hidden rounded-xl border border-border bg-background px-6 py-7"
			style={
				{
					"--map-time-scale": timeScale,
					backgroundImage:
						"radial-gradient(hsl(var(--foreground) / 0.07) 1px, transparent 1px)",
					backgroundSize: "18px 18px",
				} as React.CSSProperties
			}
		>
			<style>{FLOW_KEYFRAMES}</style>
			{/* Three columns: gutter, the flow, gutter. The flow column is
			    as wide as its widest group. Groups size themselves from
			    their animated stage width, so storage and the connectors
			    follow in normal flow on the same frames. */}
			<div className="grid grid-cols-[1fr_auto_1fr] items-center justify-items-center">
				<Tile
					mark={<Squircle icon={faGlobe} className="bg-violet-600" />}
					title="Public internet"
					subtitle={externalHost}
					tint={FLOW.ingress.color}
					className="col-start-2"
				/>
				<Connector className="col-start-2" flow={FLOW.ingress} />
				{/* Storage sits in normal flow beside the control plane so
				    the width animation carries it along; a hidden mirror
				    on the left keeps the group centered in the column. */}
				<div className="col-start-2 flex items-center">
					<div className="invisible" aria-hidden="true">
						{storage}
					</div>
					<Group
						mark={<RivetMark />}
						title="Control Plane"
						nodes={controlPlane}
						timeScale={timeScale}
					/>
					{storage}
				</div>
				<Connector className="col-start-2" flow={FLOW.actors} />
				<Group
					mark={<BrandMark file="actors-mark.svg" />}
					title="Actor Runners"
					nodes={runners}
					tint={FLOW.actors.color}
					className="col-start-2"
					timeScale={timeScale}
					onRolled={celebration.burst}
				/>
			</div>
			<canvas
				ref={celebration.canvas}
				className="pointer-events-none absolute inset-0 size-full"
			/>
		</div>
	);
}

// ---------------------------------------------------------------------------
// Motion
// ---------------------------------------------------------------------------

/** Vercel-style easing: fast start, long soft settle, no overshoot. */
const EASE_OUT = cubicBezier(0.16, 1, 0.3, 1);
const EASE_CSS = "cubic-bezier(0.16, 1, 0.3, 1)";
/** A stack opening, closing, or shifting; and the group width with it. */
const STACK_MS = 650;
/** A layer stepping in behind the front card. */
const LAYER_MS = 400;

/**
 * A small, quiet burst of confetti from the runners' front card when a
 * rollout finishes. Drawn on the map's own canvas so it stays inside the
 * map instead of raining over the page.
 */
function useCelebration() {
	const canvas = useRef<HTMLCanvasElement>(null);
	const fire = useRef<confetti.CreateTypes | null>(null);
	const burst = (card: DOMRect) => {
		const el = canvas.current;
		if (!el) return;
		fire.current ??= confetti.create(el, {
			resize: true,
			useWorker: false,
		});
		const bounds = el.getBoundingClientRect();
		const origin = {
			x: (card.left + card.width / 2 - bounds.left) / bounds.width,
			y: (card.top - bounds.top) / bounds.height,
		};
		const shared = {
			origin,
			colors: ["#38bdf8", "#8b5cf6", "#34d399", "#f8fafc"],
			scalar: 0.5,
			gravity: 0.7,
			decay: 0.9,
			ticks: 110,
			disableForReducedMotion: true,
		};
		fire.current({
			...shared,
			particleCount: 14,
			spread: 70,
			startVelocity: 13,
			angle: 90,
		});
		fire.current({
			...shared,
			particleCount: 8,
			spread: 40,
			startVelocity: 16,
			angle: 60,
		});
		fire.current({
			...shared,
			particleCount: 8,
			spread: 40,
			startVelocity: 16,
			angle: 120,
		});
	};
	return { canvas, burst };
}

// ---------------------------------------------------------------------------
// Traffic along connectors
// ---------------------------------------------------------------------------

interface Flow {
	color: string;
	/** Steady streams run continuously; bursts travel then rest. */
	mode: "steady" | "burst";
	/** `both` sends a second particle the other way. */
	direction: "forward" | "both";
}

const FLOW: Record<"ingress" | "storage" | "actors", Flow> = {
	ingress: { color: "#8b5cf6", mode: "steady", direction: "forward" },
	storage: { color: "#ef4444", mode: "burst", direction: "forward" },
	actors: { color: "#38bdf8", mode: "steady", direction: "both" },
};

/**
 * Each tile and group is washed with the colour of the traffic it handles:
 * a faint gradient from the top plus a 1px highlight on the top edge, the
 * way raised cards catch light. Alpha is low enough to work on both themes.
 */
function tinted(color: string | undefined): React.CSSProperties {
	return {
		backgroundImage: color
			? `linear-gradient(180deg, ${color}1f 0%, ${color}08 45%, transparent 100%)`
			: "linear-gradient(180deg, hsl(var(--foreground) / 0.035) 0%, transparent 60%)",
		"--card-highlight": color
			? `${color}40`
			: "hsl(var(--foreground) / 0.08)",
	} as React.CSSProperties;
}
/** Draws the top-edge highlight in a pseudo-element so it stacks with the
 * drop shadow classes instead of replacing them. */
const LIGHT =
	"before:pointer-events-none before:absolute before:inset-0 before:rounded-[inherit] before:shadow-[inset_0_1px_0_var(--card-highlight)]";

/** Length of a connector line between its two end dots. */
const CONNECTOR_PX = 40;

/**
 * Packets are soft bulges in the wire: a short gradient streak, brightest
 * at its centre, that slides along the line's axis with a fade at each end
 * so it appears to leave one dot and arrive at the other. Bursts spend most
 * of their cycle parked and invisible.
 */
const PACKET_LEN = 18;
const PACKET_WIDTH = 3;
/**
 * The end dots light up in the flow's colour as a packet leaves (`send`)
 * and as one arrives (`receive`). They run on the same cycle and delay as
 * the packet, so the keyframe percentages line up with the packet's own:
 * steady packets travel the whole cycle, bursts only its first 30%.
 */
const FLOW_KEYFRAMES = `
@keyframes map-fade-in {
	from { opacity: 0; transform: translateY(3px); }
	to { opacity: 1; transform: none; }
}
@keyframes map-flow-steady {
	0% { transform: translate(var(--fx0), var(--fy0)); opacity: 0; }
	12% { opacity: 1; }
	88% { opacity: 1; }
	100% { transform: translate(var(--fx1), var(--fy1)); opacity: 0; }
}
@keyframes map-flow-burst {
	0% { transform: translate(var(--fx0), var(--fy0)); opacity: 0; }
	4% { opacity: 1; }
	26% { opacity: 1; }
	30% { transform: translate(var(--fx1), var(--fy1)); opacity: 0; }
	100% { transform: translate(var(--fx1), var(--fy1)); opacity: 0; }
}
@keyframes map-dot-steady-send {
	0% { opacity: 1; }
	8% { opacity: 1; }
	26% { opacity: 0; }
	90% { opacity: 0; }
	100% { opacity: 1; }
}
@keyframes map-dot-steady-receive {
	0% { opacity: 1; }
	14% { opacity: 0; }
	84% { opacity: 0; }
	94% { opacity: 1; }
	100% { opacity: 1; }
}
@keyframes map-dot-burst-send {
	0% { opacity: 1; }
	5% { opacity: 1; }
	16% { opacity: 0; }
	97% { opacity: 0; }
	100% { opacity: 1; }
}
@keyframes map-dot-burst-receive {
	0% { opacity: 0; }
	24% { opacity: 0; }
	29% { opacity: 1; }
	33% { opacity: 1; }
	44% { opacity: 0; }
	100% { opacity: 0; }
}
`;

/** One packet's timing; its end dots share it so they light in step. */
interface Pulse {
	flow: Flow;
	/** Runs from the far dot back to the near one. */
	reverse: boolean;
	delayMs: number;
}

function pulseTiming(pulse: Pulse): React.CSSProperties {
	const cycleS = pulse.flow.mode === "steady" ? 2.2 : 6;
	return {
		animationDuration: `calc(${cycleS}s / var(--map-time-scale, 1))`,
		animationDelay: `calc(${pulse.delayMs}ms / var(--map-time-scale, 1))`,
		animationIterationCount: "infinite",
	};
}

function Particle({
	pulse,
	horizontal,
}: {
	pulse: Pulse;
	horizontal: boolean;
}) {
	const { flow, reverse } = pulse;
	// Travel so the streak's centre runs from one end dot to the other.
	const from = `${-PACKET_LEN / 2}px`;
	const to = `${CONNECTOR_PX - PACKET_LEN / 2}px`;
	const [start, end] = reverse ? [to, from] : [from, to];
	const axis = horizontal ? "90deg" : "180deg";
	return (
		<span
			aria-hidden="true"
			className="pointer-events-none absolute left-0 top-0 rounded-full"
			style={
				{
					...pulseTiming(pulse),
					width: horizontal ? PACKET_LEN : PACKET_WIDTH,
					height: horizontal ? PACKET_WIDTH : PACKET_LEN,
					marginLeft: horizontal ? 0 : -(PACKET_WIDTH - 1) / 2,
					marginTop: horizontal ? -(PACKET_WIDTH - 1) / 2 : 0,
					"--fx0": horizontal ? start : "0px",
					"--fx1": horizontal ? end : "0px",
					"--fy0": horizontal ? "0px" : start,
					"--fy1": horizontal ? "0px" : end,
					background: `linear-gradient(${axis}, transparent, ${flow.color} 50%, transparent)`,
					boxShadow: `0 0 6px ${flow.color}99`,
					opacity: 0,
					animationName:
						flow.mode === "steady"
							? "map-flow-steady"
							: "map-flow-burst",
					animationTimingFunction:
						flow.mode === "steady"
							? "linear"
							: "cubic-bezier(0.4, 0, 0.6, 1)",
				} as React.CSSProperties
			}
		/>
	);
}

/**
 * An end dot of a connector. It is grey at rest and carries one coloured
 * overlay per packet that touches it, lit while that packet leaves or
 * arrives. `far` is the dot at the connector's end rather than its start.
 */
function EndDot({ pulses, far }: { pulses: Pulse[]; far: boolean }) {
	return (
		<span className="relative size-1.5 shrink-0 rounded-full bg-muted-foreground/50">
			{pulses.map((pulse, i) => {
				// A reversed packet starts at the far dot.
				const sends = pulse.reverse === far;
				return (
					<span
						// biome-ignore lint/suspicious/noArrayIndexKey: static list
						key={i}
						aria-hidden="true"
						className="absolute inset-0 rounded-full"
						style={{
							...pulseTiming(pulse),
							backgroundColor: pulse.flow.color,
							boxShadow: `0 0 6px 1px ${pulse.flow.color}aa`,
							opacity: 0,
							animationName: `map-dot-${pulse.flow.mode}-${sends ? "send" : "receive"}`,
							animationTimingFunction: "ease-in-out",
						}}
					/>
				);
			})}
		</span>
	);
}

/**
 * A short line with a dot at each end and traffic pulsing along it.
 * Vertical between rows of the flow; horizontal (with the attached tile as
 * children) off to the side.
 */
function Connector({
	horizontal = false,
	flow,
	className,
	children,
}: {
	horizontal?: boolean;
	flow: Flow;
	className?: string;
	children?: ReactNode;
}) {
	const pulses: Pulse[] = [{ flow, reverse: false, delayMs: 0 }];
	if (flow.mode === "steady") {
		// A second packet half a cycle behind: the other way for two-way
		// traffic, the same way for a steady stream.
		pulses.push({
			flow,
			reverse: flow.direction === "both",
			delayMs: 1100,
		});
	}
	return (
		<div
			className={cn(
				"flex items-center",
				horizontal ? "flex-row" : "flex-col",
				className,
			)}
		>
			<EndDot pulses={pulses} far={false} />
			<div
				className={cn(
					"relative bg-muted-foreground/30",
					horizontal ? "h-px" : "w-px",
				)}
				style={
					horizontal
						? { width: CONNECTOR_PX }
						: { height: CONNECTOR_PX }
				}
			>
				{pulses.map((pulse, i) => (
					<Particle
						// biome-ignore lint/suspicious/noArrayIndexKey: static list
						key={i}
						pulse={pulse}
						horizontal={horizontal}
					/>
				))}
			</div>
			<EndDot pulses={pulses} far />
			{children}
		</div>
	);
}

// ---------------------------------------------------------------------------
// Marks and surfaces
// ---------------------------------------------------------------------------

/**
 * Marks follow the brand squircles (128 grid: outer rx 44, inner outlined
 * square inset 18.25 with rx 25.75). Product marks ship with that shape
 * baked in; the other tiles draw it here so all four match. A faint ring
 * lifts the marks off the card, which matters most for the black Rivet box
 * on the dark theme.
 */
const MARK_RING =
	"relative size-8 shrink-0 rounded-[34.375%] after:pointer-events-none after:absolute after:inset-0 after:rounded-[inherit] after:ring-1 after:ring-inset after:ring-foreground/20";

function Squircle({ icon, className }: { icon: IconProp; className: string }) {
	return (
		<span className={cn(MARK_RING, "block text-white", className)}>
			<span className="absolute inset-[14.25%] flex items-center justify-center rounded-[28%] border-2 border-white/90">
				<Icon icon={icon} className="size-3" />
			</span>
		</span>
	);
}

function BrandMark({ file }: { file: string }) {
	return (
		<span className={cn(MARK_RING, "block")}>
			<img
				src={publicUrl(`images/brand/${file}`)}
				alt=""
				aria-hidden="true"
				className="size-full"
				draggable={false}
			/>
		</span>
	);
}

function RivetMark() {
	return (
		<span className={cn(MARK_RING, "block")}>
			<LogoMark className="size-full" />
		</span>
	);
}

/**
 * Dark theme card and background are nearly the same lightness, so map
 * surfaces step up in lightness instead: groups and tiles sit a little
 * above the canvas, version cards a little above their group. Shadows stay
 * soft and offset-free so they also fall on the stack layers up and to the
 * right without reading as a glow.
 */
const SURFACE = "bg-card dark:bg-[hsl(240_6%_9%)]";
const CARD_SURFACE = "bg-card dark:bg-[hsl(240_6%_13%)]";
const SHADOW_SM =
	"shadow-[0_1px_2px_rgba(0,0,0,0.04)] dark:shadow-[0_1px_2px_rgba(0,0,0,0.3),0_0_6px_rgba(0,0,0,0.2)]";
const SHADOW_MD =
	"shadow-[0_1px_3px_rgba(0,0,0,0.06)] dark:shadow-[0_1px_2px_rgba(0,0,0,0.35),0_0_8px_rgba(0,0,0,0.25)]";
const STACK_BORDER =
	"border border-foreground/[0.14] dark:border-foreground/[0.18]";
/** Raised-card lighting for the version cards: a 1px highlight along the top edge. */
const STACK_LIGHT = tinted(undefined);

// ---------------------------------------------------------------------------
// Groups and version stacks
// ---------------------------------------------------------------------------

/** Front card size; every stack reserves room for its layers behind it. */
const CARD_W = 180;
const CARD_H = 60;
/** How many card layers to draw behind the front card for a stack. */
const MAX_STACK_LAYERS = 4;
const LAYER_OFFSET_PX = 6;
const FOOT_PX = (MAX_STACK_LAYERS - 1) * LAYER_OFFSET_PX;
const STACK_W = CARD_W + FOOT_PX;
const STACK_H = CARD_H + FOOT_PX;
const STACK_GAP = 16;
const stackX = (i: number) => i * (STACK_W + STACK_GAP);

/**
 * All nodes of one kind, grouped by the version they run. Each version is a
 * 3D stack of cards, one layer per node, so a rolling update shows the new
 * version's stack growing beside the old one.
 *
 * Stacks sit on an absolutely positioned stage. A stack rolling in slides
 * out from behind the one before it; one rolling out slides back under its
 * replacement; the stage width and the stacks that shift to make room all
 * tween on the same clock, so nothing in the group jumps.
 */
function Group({
	mark,
	title,
	nodes,
	tint,
	className,
	timeScale,
	onRolled,
}: {
	mark: ReactNode;
	title: string;
	nodes: ClusterNode[];
	tint?: string;
	className?: string;
	timeScale: number;
	/** Fires with the surviving front card once an old version is gone. */
	onRolled?: (card: DOMRect) => void;
}) {
	// Nodes are listed old version first, so insertion order puts the
	// outgoing version on the left and the one rolling in on the right.
	const versions = [...new Set(nodes.map((n) => n.version))];
	const rolling = versions.length > 1;
	const cards = useRef(new Map<string, HTMLDivElement>());
	// A stack on its way out has no nodes left; it keeps showing its last.
	const lastNodes = useRef(new Map<string, ClusterNode[]>());
	for (const version of versions) {
		lastNodes.current.set(
			version,
			nodes.filter((n) => n.version === version),
		);
	}

	const targets = new Map<string, Pose>();
	targets.set("stage", { w: stackX(versions.length - 1) + STACK_W });
	versions.forEach((version, i) => {
		targets.set(`stack:${version}`, { x: stackX(i), o: 1 });
	});
	const [options] = useState<SceneOptions>(() => ({
		duration: STACK_MS,
		ease: EASE_OUT,
		// Tucked behind the stack to its left, so it emerges as it travels.
		enter: (_key, to) => ({ x: to.x - (STACK_W + STACK_GAP) + 28, o: 0 }),
		// Back under its neighbour on the right.
		exit: (_key, last) => ({ x: last.x + 40, o: 0 }),
	}));
	options.onExited = (key) => {
		if (!key.startsWith("stack:") || !onRolled) return;
		const survivor = cards.current.get(versions[0]);
		if (survivor) onRolled(survivor.getBoundingClientRect());
	};
	const scene = useScene(targets, options, timeScale);
	const stage = scene.get("stage")?.pose.w ?? STACK_W;

	return (
		<div
			className={cn(
				"relative rounded-xl border border-border p-3",
				LIGHT,
				SURFACE,
				SHADOW_SM,
				className,
			)}
			style={tinted(tint)}
		>
			<div className="mb-4 flex items-center gap-3">
				{mark}
				<span className="truncate text-sm font-semibold">{title}</span>
			</div>
			<div className="relative" style={{ width: stage, height: STACK_H }}>
				{[...scene]
					.filter(([key]) => key.startsWith("stack:"))
					.map(([key, entity]) => {
						const version = key.slice("stack:".length);
						const i = versions.indexOf(version);
						const stackNodes = lastNodes.current.get(version) ?? [];
						return (
							<VersionStack
								key={version}
								ref={(el) => {
									if (el) cards.current.set(version, el);
									else cards.current.delete(version);
								}}
								version={version}
								nodes={stackNodes}
								x={entity.pose.x}
								opacity={entity.pose.o}
								// Travelling stacks pass beneath settled ones.
								raised={entity.phase === "present"}
								timeScale={timeScale}
								// During a rollout the outgoing version drains
								// and the incoming one fills, whatever each node
								// is doing this instant, so the labels hold
								// steady instead of flipping between batches.
								phase={
									i < 0 || (rolling && i === 0)
										? "terminating"
										: rolling
											? "provisioning"
											: undefined
								}
							/>
						);
					})}
			</div>
		</div>
	);
}

const NODE_STATUS: Record<
	NodeStatus,
	{ label: string; dot: string; text: string }
> = {
	provisioning: {
		label: "Starting",
		dot: "bg-sky-500 animate-pulse",
		text: "text-sky-600 dark:text-sky-400",
	},
	ready: {
		label: "Ready",
		dot: "bg-emerald-500 shadow-[0_0_5px_theme(colors.emerald.500/0.8)]",
		text: "text-emerald-600 dark:text-emerald-400",
	},
	terminating: {
		label: "Terminating",
		dot: "bg-amber-500 animate-pulse",
		text: "text-amber-600 dark:text-amber-400",
	},
};

/** "4 ready · 2 starting · 1 terminating", for the status tooltip. */
function describeNodes(nodes: ClusterNode[]): string {
	return (["ready", "provisioning", "terminating"] as const)
		.map((status) => ({
			status,
			n: nodes.filter((node) => node.status === status).length,
		}))
		.filter(({ n }) => n > 0)
		.map(
			({ status, n }) =>
				`${n} ${NODE_STATUS[status].label.toLowerCase()}`,
		)
		.join(" · ");
}

const LAYER_OPTIONS: SceneOptions = {
	duration: LAYER_MS,
	ease: EASE_OUT,
	// A new layer steps out from the one in front of it.
	enter: (_key, to) => ({ d: to.d - 1, o: 0 }),
	exit: (_key, last) => ({ d: last.d - 1, o: 0 }),
};

/**
 * One version's nodes as a stack of cards: one layer per node (capped), the
 * front card carrying the version, the node count, and what the stack is
 * doing. The stack has a fixed footprint whatever its depth, so layers
 * stepping in and out never move anything else.
 */
function VersionStack({
	ref,
	version,
	nodes,
	x,
	opacity,
	raised,
	timeScale,
	phase,
}: {
	ref: (el: HTMLDivElement | null) => void;
	version: string;
	nodes: ClusterNode[];
	x: number;
	opacity: number;
	raised: boolean;
	timeScale: number;
	/** Forced while a rollout is under way; otherwise read off the nodes. */
	phase?: NodeStatus;
}) {
	const kind: NodeStatus =
		phase ??
		(nodes.some((n) => n.status === "provisioning")
			? "provisioning"
			: "ready");
	const status = NODE_STATUS[kind];
	const depth = Math.min(nodes.length, MAX_STACK_LAYERS) - 1;
	const targets = new Map<string, Pose>();
	for (let d = 1; d <= depth; d++) targets.set(`layer:${d}`, { d, o: 1 });
	const layers = useScene(targets, LAYER_OPTIONS, timeScale);
	return (
		<div
			className="absolute bottom-0 left-0"
			style={{
				width: STACK_W,
				height: STACK_H,
				transform: `translate3d(${x}px, 0, 0)`,
				opacity,
				zIndex: raised ? 1 : 0,
			}}
			data-stack={version}
		>
			{[...layers].map(([key, layer]) => (
				<div
					key={key}
					className={cn(
						"absolute bottom-0 left-0 rounded-lg",
						LIGHT,
						STACK_BORDER,
						CARD_SURFACE,
						SHADOW_SM,
					)}
					style={{
						...STACK_LIGHT,
						width: CARD_W,
						height: CARD_H,
						// Each layer steps up and to the right; nearer layers
						// paint over farther ones and cards are opaque.
						transform: `translate3d(${layer.pose.d * LAYER_OFFSET_PX}px, ${-layer.pose.d * LAYER_OFFSET_PX}px, 0)`,
						opacity: layer.pose.o,
						zIndex: MAX_STACK_LAYERS - Math.round(layer.pose.d),
					}}
				/>
			))}
			<div
				ref={ref}
				className={cn(
					"absolute bottom-0 left-0 z-10 flex flex-col justify-center gap-1.5 rounded-lg px-3",
					LIGHT,
					STACK_BORDER,
					CARD_SURFACE,
					SHADOW_MD,
				)}
				style={{ ...STACK_LIGHT, width: CARD_W, height: CARD_H }}
			>
				<div className="flex items-baseline justify-between gap-2">
					<span className="whitespace-nowrap font-mono text-sm font-semibold">
						{version}
					</span>
					<span className="whitespace-nowrap text-[11px] tabular-nums text-muted-foreground">
						{nodes.length} {nodes.length === 1 ? "node" : "nodes"}
					</span>
				</div>
				<WithTooltip
					content={describeNodes(nodes)}
					trigger={
						<span
							key={kind}
							className={cn(
								"flex w-fit cursor-default items-center gap-1.5 text-[11px]",
								status.text,
							)}
							style={{
								animation: `map-fade-in calc(0.2s / var(--map-time-scale, 1)) ${EASE_CSS} both`,
							}}
						>
							<span
								className={cn(
									"size-1.5 rounded-full",
									status.dot,
								)}
							/>
							{status.label}
						</span>
					}
				/>
			</div>
		</div>
	);
}

function Tile({
	mark,
	title,
	subtitle,
	tint,
	className,
}: {
	mark: ReactNode;
	title: string;
	subtitle?: ReactNode;
	tint?: string;
	className?: string;
}) {
	return (
		<div
			className={cn(
				"relative flex items-center gap-3 rounded-xl border border-border px-4 py-3 text-left",
				LIGHT,
				SURFACE,
				SHADOW_SM,
				className,
			)}
			style={tinted(tint)}
		>
			{mark}
			<span className="flex min-w-0 flex-1 flex-col">
				<span className="truncate text-sm font-semibold">{title}</span>
				{subtitle !== undefined ? (
					<span className="truncate text-xs text-muted-foreground">
						{subtitle}
					</span>
				) : null}
			</span>
		</div>
	);
}
