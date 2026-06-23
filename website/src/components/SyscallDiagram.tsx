import {
	faBolt,
	faCloud,
	faFolderTree,
	faGears,
	faPlug,
	faRobot,
	faSitemap,
	faSquareBinary,
} from "@rivet-gg/icons";
import { useEffect, useRef, useState } from "react";

// Scroll-driven companion diagram for the "Linux on WebAssembly" post. A single
// SVG pins to the top of the viewport while the post's sections scroll beneath
// it. Each section drops a `<div data-syscall-step="N" />` anchor; as an anchor
// crosses the middle of the viewport the diagram morphs to that build step, so
// the picture and the prose advance together.
//
// Layout is content-driven: step 1 is a lone wasm module, and each later step
// adds only the layers that exist yet, so early steps stay compact instead of
// reserving empty space. Pieces are rendered as keyed React <g> nodes, so when
// the step changes React preserves the nodes that carry over and only the newly
// added pieces mount and play the enter animation.
//
// Color coding: ink = the program you write (guest, agent VMs, orchestration),
// pine = the syscall surface we add (chips, v8), sage = kernel-owned machinery
// (process/file/socket tables, S3, the shared kernel boundary).

const TITLES: Record<number, string> = {
	0: "architecture",
	1: "bare guest",
	2: "processes",
	3: "filesystem",
	4: "network",
	5: "js acceleration",
	6: "agents",
	7: "agents",
};

type Piece = { key: string; el: string };

// Embed a Font Awesome glyph as an SVG <path>, scaled to `size` (height in
// viewBox units) and centered on (cx, cy). FA icon data is [w, h, , , pathData].
type FaIcon = { icon: [number, number, unknown, unknown, string | string[]] };
function icon(fa: FaIcon, cx: number, cy: number, size: number, cls: string): string {
	const [w, h, , , raw] = fa.icon;
	const d = Array.isArray(raw) ? raw[raw.length - 1] : raw;
	const s = size / h;
	const tx = cx - (w * s) / 2;
	const ty = cy - (h * s) / 2;
	return `<g class="${cls}" transform="translate(${tx} ${ty}) scale(${s})"><path d="${d}"/></g>`;
}

function buildSingle(step: number): { vb: string; pieces: Piece[] } {
	const VM_X = 190;
	const VM_W = 300;
	const IN_X = 206;
	const IN_W = 268;
	const PAD = 14;
	const GAP = 12;
	const vmTop = 30;
	const guestY = vmTop + PAD;
	const guestH = 40;

	// The wasm module itself. Identical markup in step 1 and later, so React keeps
	// the node and it stays put while the OS materializes around it.
	const guest =
		`<g class="c-ink"><rect x="${IN_X}" y="${guestY}" width="${IN_W}" height="${guestH}" rx="8" stroke-width="0.5"/>` +
		icon(faSquareBinary, IN_X + 26, guestY + guestH / 2, 16, "ic-ink") +
		`<text class="th" x="${IN_X + 46}" y="${guestY + guestH / 2}" text-anchor="start" dominant-baseline="central">wasm guest</text></g>`;

	// Step 1: a bare wasm module, nothing around it yet.
	if (step === 1) {
		return { vb: "176 12 328 96", pieces: [{ key: "guest", el: guest }] };
	}

	const chips = [
		{ k: "wasip1", l: "wasip1" },
		{ k: "host_process", l: "host_process" },
	];
	if (step >= 3) chips.push({ k: "path_open", l: "path_open" });
	if (step >= 4) chips.push({ k: "host_net", l: "host_net" });
	const rows = Math.ceil(chips.length / 2);
	const SY = guestY + guestH + GAP;
	const SH = 18 + rows * 26 + 6;

	const tables: { k: string; label: string; ic: FaIcon }[] = [
		{ k: "proc", label: "process", ic: faGears },
	];
	if (step >= 3) tables.push({ k: "vfs", label: "files", ic: faFolderTree });
	if (step >= 4) tables.push({ k: "sock", label: "sockets", ic: faPlug });
	const KY = SY + SH + GAP;
	const KH = 78;
	const vmBottom = KY + KH + PAD;
	const vmH = vmBottom - vmTop;
	const hasFS = step >= 3;
	const s3Y = vmBottom + 16;
	const s3H = 30;
	const h = (hasFS ? s3Y + s3H : vmBottom) + 14;

	const pieces: Piece[] = [];
	pieces.push({
		key: "vm",
		el: `<rect class="box dash" x="${VM_X}" y="${vmTop}" width="${VM_W}" height="${vmH}" rx="14" stroke-dasharray="5 4"/><text class="tl" x="${VM_X + 12}" y="${vmTop - 7}">agentOS VM</text>`,
	});
	pieces.push({ key: "guest", el: guest });
	if (step >= 5) {
		pieces.push({
			key: "v8",
			el:
				`<g class="c-pine"><rect x="${402}" y="${guestY + 8}" width="64" height="24" rx="6" stroke-width="0.5"/>` +
				icon(faBolt, 420, guestY + 20, 12, "ic-pine") +
				`<text class="ts" x="436" y="${guestY + 20}" text-anchor="middle" dominant-baseline="central">v8</text></g>`,
		});
	}
	pieces.push({
		key: "sbox",
		el: `<rect class="box" x="${IN_X}" y="${SY}" width="${IN_W}" height="${SH}" rx="9"/><text class="tl" x="${IN_X + 10}" y="${SY + 12}">syscalls</text>`,
	});
	chips.forEach((c, i) => {
		const col = i % 2;
		const row = Math.floor(i / 2);
		const x = 214 + col * 128;
		const y = SY + 18 + row * 26;
		pieces.push({
			key: `chip-${c.k}`,
			el: `<g class="c-pine"><rect x="${x}" y="${y}" width="120" height="22" rx="5" stroke-width="0.5"/><text class="tm" x="${x + 60}" y="${y + 11}" text-anchor="middle" dominant-baseline="central">${c.l}</text></g>`,
		});
	});
	pieces.push({
		key: "kbox",
		el: `<rect class="box" x="${IN_X}" y="${KY}" width="${IN_W}" height="${KH}" rx="9"/><text class="tl" x="${IN_X + 10}" y="${KY + 12}">kernel</text>`,
	});
	tables.forEach((t, i) => {
		const x = 212 + i * 88;
		const y = KY + 18;
		pieces.push({
			key: `tbl-${t.k}`,
			el:
				`<g class="c-sage"><rect x="${x}" y="${y}" width="80" height="50" rx="8" stroke-width="0.5"/>` +
				icon(t.ic, x + 40, y + 18, 16, "ic-sage") +
				`<text class="ts" x="${x + 40}" y="${y + 39}" text-anchor="middle" dominant-baseline="central">${t.label}</text></g>`,
		});
	});
	if (hasFS) {
		const vfsX = 212 + 88 + 40;
		pieces.push({
			key: "s3",
			el:
				`<line x1="${vfsX}" y1="${KY + 68}" x2="${vfsX}" y2="${s3Y}" class="arr" marker-end="url(#sc-a)"/>` +
				`<g class="c-sage"><rect x="${vfsX - 42}" y="${s3Y}" width="84" height="${s3H}" rx="8" stroke-width="0.5"/>` +
				icon(faCloud, vfsX - 14, s3Y + s3H / 2, 13, "ic-sage") +
				`<text class="ts" x="${vfsX + 12}" y="${s3Y + s3H / 2}" text-anchor="middle" dominant-baseline="central">S3</text></g>`,
		});
	}
	return { vb: `150 0 380 ${h}`, pieces };
}

function buildFleet(): { vb: string; pieces: Piece[] } {
	const tile = (x: number, sub: string) =>
		`<g class="c-ink"><rect x="${x}" y="96" width="150" height="100" rx="10" stroke-width="0.5"/>` +
		icon(faRobot, x + 75, 121, 21, "ic-ink") +
		`<text class="th" x="${x + 75}" y="148" text-anchor="middle" dominant-baseline="central">agent VM</text><text class="ts" x="${x + 75}" y="165" text-anchor="middle" dominant-baseline="central">proc · fs · net</text><text class="ts" x="${x + 75}" y="181" text-anchor="middle" dominant-baseline="central">${sub}</text></g>`;
	const pieces: Piece[] = [
		{
			key: "orch",
			el:
				`<g class="c-ink"><rect x="180" y="20" width="320" height="44" rx="10" stroke-width="0.5"/>` +
				icon(faSitemap, 224, 42, 17, "ic-ink") +
				`<text class="th" x="346" y="36" text-anchor="middle" dominant-baseline="central">orchestration</text><text class="ts" x="346" y="52" text-anchor="middle" dominant-baseline="central">session router</text></g>`,
		},
		{ key: "tile1", el: tile(60, "the VM you built") },
		{ key: "tile2", el: tile(265, "its own isolate") },
		{ key: "tile3", el: tile(470, "its own isolate") },
		{
			key: "arrows-down",
			el: `<line x1="320" y1="64" x2="135" y2="96" class="arr" marker-end="url(#sc-a)"/><line x1="340" y1="64" x2="340" y2="96" class="arr" marker-end="url(#sc-a)"/><line x1="360" y1="64" x2="545" y2="96" class="arr" marker-end="url(#sc-a)"/>`,
		},
		{
			key: "arrows-up",
			el: `<line x1="135" y1="196" x2="135" y2="224" class="arr"/><line x1="340" y1="196" x2="340" y2="224" class="arr"/><line x1="545" y1="196" x2="545" y2="224" class="arr"/>`,
		},
		{
			key: "kernel-surface",
			el:
				`<rect class="c-sage-fill" x="60" y="224" width="560" height="42" rx="9" stroke-width="0.5"/>` +
				`<text class="th" x="340" y="240" text-anchor="middle" dominant-baseline="central">shared kernel surface</text><text class="ts" x="340" y="256" text-anchor="middle" dominant-baseline="central">one proc · fs · net boundary</text>`,
		},
	];
	return { vb: "40 0 600 282", pieces };
}

// The starting frame, shown before the first build step scrolls into view: the
// full single-VM architecture with every label stripped. It is a quiet preview of
// where the build ends up. Reuse the most complete single-VM frame and remove the
// text so only the boxes and icons remain.
function buildPreview(): { vb: string; pieces: Piece[] } {
	const { vb, pieces } = buildSingle(5);
	const stripped = pieces.map((p) => ({
		key: p.key,
		el: p.el.replace(/<text\b[^>]*>.*?<\/text>/g, ""),
	}));
	return { vb, pieces: stripped };
}

function parseVb(vb: string): number[] {
	return vb.split(" ").map(Number);
}

export function SyscallDiagram() {
	// Step 0 is the unlabeled preview frame shown above the first build step.
	const [step, setStep] = useState(0);

	useEffect(() => {
		const nodes = Array.from(
			document.querySelectorAll<HTMLElement>("[data-syscall-step]"),
		);
		const anchors = nodes
			.map((el) => ({ el, step: Number.parseInt(el.dataset.syscallStep ?? "1", 10) }))
			.filter((a) => Number.isFinite(a.step));
		if (anchors.length === 0) return;

		let raf = 0;
		const update = () => {
			raf = 0;
			const line = window.innerHeight * 0.5;
			let next = 0;
			for (const a of anchors) {
				if (a.el.getBoundingClientRect().top <= line) next = a.step;
			}
			setStep((prev) => (prev === next ? prev : next));
		};
		const onScroll = () => {
			if (!raf) raf = requestAnimationFrame(update);
		};
		update();
		window.addEventListener("scroll", onScroll, { passive: true });
		window.addEventListener("resize", onScroll);
		return () => {
			window.removeEventListener("scroll", onScroll);
			window.removeEventListener("resize", onScroll);
			if (raf) cancelAnimationFrame(raf);
		};
	}, []);

	const { vb, pieces } = step === 0 ? buildPreview() : step <= 5 ? buildSingle(step) : buildFleet();
	const title = TITLES[step] ?? "";
	const isFleet = step >= 6;

	// viewBox is not CSS-transitionable, so animate the zoom/pan between steps by
	// interpolating it per frame with rAF instead of letting it snap. Within the
	// single-VM view (steps 1-5) this is a smooth zoom. Crossing into the fleet
	// view is a different coordinate system, so interpolating it would fly the new
	// content in from a wrong frame: snap instead and let the piece fade-in cover.
	const [vbAnim, setVbAnim] = useState(vb);
	const vbCurrentRef = useRef(vb);
	const rafRef = useRef(0);
	const prevFleetRef = useRef(isFleet);

	useEffect(() => {
		const from = parseVb(vbCurrentRef.current);
		const to = parseVb(vb);
		const modeChanged = prevFleetRef.current !== isFleet;
		prevFleetRef.current = isFleet;
		const reduce =
			typeof window !== "undefined" &&
			window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
		if (reduce || modeChanged || from.length !== 4 || from.every((v, i) => v === to[i])) {
			cancelAnimationFrame(rafRef.current);
			vbCurrentRef.current = vb;
			setVbAnim(vb);
			return;
		}
		const DURATION = 450;
		let start = 0;
		cancelAnimationFrame(rafRef.current);
		const tick = (t: number) => {
			if (!start) start = t;
			const p = Math.min(1, (t - start) / DURATION);
			// easeInOutCubic
			const e = p < 0.5 ? 4 * p * p * p : 1 - (-2 * p + 2) ** 3 / 2;
			const next = from.map((f, i) => f + (to[i] - f) * e).join(" ");
			vbCurrentRef.current = next;
			setVbAnim(next);
			if (p < 1) rafRef.current = requestAnimationFrame(tick);
		};
		rafRef.current = requestAnimationFrame(tick);
		return () => cancelAnimationFrame(rafRef.current);
	}, [vb, isFleet]);

	return (
		<div className="not-prose syscall-diagram">
			<p className="cap">{title}</p>
			<svg viewBox={vbAnim} role="img" aria-label={`Diagram: ${title}`}>
				<defs>
					<marker
						id="sc-a"
						viewBox="0 0 10 10"
						refX="8"
						refY="5"
						markerWidth="6"
						markerHeight="6"
						orient="auto-start-reverse"
					>
						<path
							d="M2 1L8 5L2 9"
							fill="none"
							stroke="context-stroke"
							strokeWidth="1.5"
							strokeLinecap="round"
							strokeLinejoin="round"
						/>
					</marker>
				</defs>
				{pieces.map((p) => (
					// biome-ignore lint/security/noDangerouslySetInnerHtml: SVG markup is generated from a fixed, non-user template.
					<g key={p.key} className="pc" dangerouslySetInnerHTML={{ __html: p.el }} />
				))}
			</svg>
			<style>{`
				.syscall-diagram {
					padding: 0.7rem 0.85rem 0.55rem;
					background: #EFEFEF;
					border: 1px solid rgb(27 25 22 / 0.08);
					border-radius: 0.875rem;
				}
				.syscall-diagram svg {
					display: block;
					width: 100%;
					height: 16rem;
					max-height: none;
					margin: 0 auto;
				}
				@media (min-width: 768px) {
					.syscall-diagram svg { max-width: 30rem; height: 19rem; }
				}
				/* Structural boxes: hairline frames, no fill. */
				.syscall-diagram .box { fill: none; stroke: #1b1916; stroke-opacity: .16; }
				.syscall-diagram .dash { stroke-opacity: .22; }
				/* Text roles: th = title, ts = supporting, tm = mono chip, tl = eyebrow label. */
				.syscall-diagram .th { fill: #1b1916; font: 600 13px Manrope, ui-sans-serif, system-ui, sans-serif; }
				.syscall-diagram .ts { fill: #56524a; font: 500 11px Manrope, ui-sans-serif, system-ui, sans-serif; }
				.syscall-diagram .tm { font: 500 10.5px "JetBrains Mono", ui-monospace, monospace; letter-spacing: -.01em; }
				.syscall-diagram .tl { fill: #9a948a; font: 600 8.5px "JetBrains Mono", ui-monospace, monospace; letter-spacing: .12em; text-transform: uppercase; }
				.syscall-diagram .arr { stroke: #2e4034; stroke-width: 1.2; fill: none; opacity: .8; }
				/* Ink = the program you write (guest, agent VMs, orchestration). */
				.syscall-diagram .c-ink rect { fill: #1b1916; fill-opacity: .055; stroke: #1b1916; stroke-opacity: .32; }
				.syscall-diagram .c-ink text { fill: #1b1916; }
				/* Pine = the syscall surface we add (chips, v8). */
				.syscall-diagram .c-pine rect { fill: #2e4034; fill-opacity: .08; stroke: #2e4034; stroke-opacity: .6; }
				.syscall-diagram .c-pine text { fill: #2e4034; }
				/* Sage = kernel-owned machinery (tables, S3, shared kernel surface). */
				.syscall-diagram .c-sage rect { fill: #93a286; fill-opacity: .22; stroke: #6f7d63; stroke-opacity: .65; }
				.syscall-diagram .c-sage text { fill: #4f5a46; }
				.syscall-diagram .c-sage-fill { fill: #93a286; fill-opacity: .22; stroke: #6f7d63; stroke-opacity: .45; }
				/* Icon fills, matched to their box's color. */
				.syscall-diagram .ic-ink { fill: #1b1916; fill-opacity: .7; }
				.syscall-diagram .ic-pine { fill: #2e4034; fill-opacity: .85; }
				.syscall-diagram .ic-sage { fill: #5c6953; fill-opacity: .9; }
				.syscall-diagram .cap {
					margin: .1rem 0 .5rem;
					text-align: center;
					font: 600 11px/1.3 "JetBrains Mono", ui-monospace, monospace;
					letter-spacing: .14em;
					text-transform: uppercase;
					color: #2e4034;
				}
				@keyframes sc-in { from { opacity: 0; transform: scale(.9); } to { opacity: 1; transform: scale(1); } }
				.syscall-diagram .pc {
					animation: sc-in .4s cubic-bezier(.2,.7,.3,1) both;
					transform-box: fill-box;
					transform-origin: center;
				}
				@media (prefers-reduced-motion: reduce) {
					.syscall-diagram .pc { animation: none; }
				}
			`}</style>
		</div>
	);
}
