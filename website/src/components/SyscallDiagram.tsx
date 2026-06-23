import { useEffect, useState } from "react";

// Scroll-driven companion diagram for the "Linux on WebAssembly" post. A single
// SVG pins to the top of the viewport while the post's sections scroll beneath
// it. Each section drops a `<div data-syscall-step="N" />` anchor; as an anchor
// crosses the middle of the viewport the diagram morphs to that build step, so
// the picture and the prose advance together.
//
// Layout is content-driven: each step only draws the layers that exist yet, so
// early steps stay compact instead of reserving empty space for syscalls and
// kernel tables that have not been introduced. Pieces are rendered as keyed
// React <g> nodes, so when the step changes React preserves the nodes that
// carry over and only the newly added pieces mount and play the enter
// animation, rather than the whole diagram replaying from scratch.

const TITLES: Record<number, string> = {
	1: "bare guest",
	2: "processes",
	3: "filesystem",
	4: "network",
	5: "js acceleration",
	6: "agents",
	7: "agents",
};

type Piece = { key: string; el: string };

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

	const chips = [{ k: "wasip1", l: "wasip1" }];
	if (step >= 2) chips.push({ k: "host_process", l: "host_process" });
	if (step >= 3) chips.push({ k: "path_open", l: "path_open" });
	if (step >= 4) chips.push({ k: "host_net", l: "host_net" });
	const rows = Math.ceil(chips.length / 2);
	const SY = guestY + guestH + GAP;
	const SH = 18 + rows * 26 + 6;

	const tables: { k: string; a: string; b: string }[] = [];
	if (step >= 2) tables.push({ k: "proc", a: "process", b: "table" });
	if (step >= 3) tables.push({ k: "vfs", a: "virtual", b: "fs" });
	if (step >= 4) tables.push({ k: "sock", a: "socket", b: "table" });
	const KY = SY + SH + GAP;
	const KH = tables.length ? 78 : 30;
	const vmBottom = KY + KH + PAD;
	const vmH = vmBottom - vmTop;
	const hasFS = step >= 3;
	const s3Y = vmBottom + 16;
	const s3H = 30;
	const h = (hasFS ? s3Y + s3H : vmBottom) + 14;

	const pieces: Piece[] = [];
	pieces.push({
		key: "vm",
		el: `<rect class="box dash" x="${VM_X}" y="${vmTop}" width="${VM_W}" height="${vmH}" rx="16" stroke-dasharray="5 3"/><text class="ts" x="${VM_X + 10}" y="${vmTop - 6}">agentOS VM</text>`,
	});
	pieces.push({
		key: "guest",
		el: `<g class="c-purple"><rect x="${IN_X}" y="${guestY}" width="${IN_W}" height="${guestH}" rx="8" stroke-width="0.5"/><text class="th" x="340" y="${guestY + guestH / 2}" text-anchor="middle" dominant-baseline="central">wasm guest</text></g>`,
	});
	if (step >= 5) {
		pieces.push({
			key: "v8",
			el: `<g class="c-teal"><rect x="402" y="${guestY + 8}" width="64" height="24" rx="6" stroke-width="0.5"/><text class="ts" x="434" y="${guestY + 20}" text-anchor="middle" dominant-baseline="central">v8 ⚡</text></g>`,
		});
	}
	pieces.push({
		key: "sbox",
		el: `<rect class="box" x="${IN_X}" y="${SY}" width="${IN_W}" height="${SH}" rx="10"/><text class="ts" x="${IN_X + 8}" y="${SY + 13}">syscall surface</text>`,
	});
	chips.forEach((c, i) => {
		const col = i % 2;
		const row = Math.floor(i / 2);
		const x = 214 + col * 128;
		const y = SY + 18 + row * 26;
		pieces.push({
			key: `chip-${c.k}`,
			el: `<g class="c-teal"><rect x="${x}" y="${y}" width="120" height="22" rx="5" stroke-width="0.5"/><text class="ts" x="${x + 60}" y="${y + 11}" text-anchor="middle" dominant-baseline="central">${c.l}</text></g>`,
		});
	});
	pieces.push({
		key: "kbox",
		el: `<rect class="box" x="${IN_X}" y="${KY}" width="${IN_W}" height="${KH}" rx="10"/><text class="ts" x="${IN_X + 8}" y="${KY + 13}">kernel</text>`,
	});
	tables.forEach((t, i) => {
		const x = 212 + i * 88;
		const y = KY + 18;
		pieces.push({
			key: `tbl-${t.k}`,
			el: `<g class="c-gray"><rect x="${x}" y="${y}" width="80" height="50" rx="7" stroke-width="0.5"/><text class="ts" x="${x + 40}" y="${y + 20}" text-anchor="middle" dominant-baseline="central">${t.a}</text><text class="ts" x="${x + 40}" y="${y + 34}" text-anchor="middle" dominant-baseline="central">${t.b}</text></g>`,
		});
	});
	if (hasFS) {
		const vfsX = 212 + 88 + 40;
		pieces.push({
			key: "s3",
			el: `<line x1="${vfsX}" y1="${KY + 68}" x2="${vfsX}" y2="${s3Y}" class="arr" marker-end="url(#sc-a)"/><rect class="box" x="${vfsX - 40}" y="${s3Y}" width="80" height="${s3H}" rx="7"/><text class="ts" x="${vfsX}" y="${s3Y + s3H / 2}" text-anchor="middle" dominant-baseline="central">S3</text>`,
		});
	}
	return { vb: `150 0 380 ${h}`, pieces };
}

function buildFleet(): { vb: string; pieces: Piece[] } {
	const tile = (x: number, sub: string) =>
		`<g class="c-teal"><rect x="${x}" y="96" width="150" height="100" rx="10" stroke-width="0.5"/><text class="th" x="${x + 75}" y="122" text-anchor="middle" dominant-baseline="central">agent VM</text><text class="ts" x="${x + 75}" y="144" text-anchor="middle" dominant-baseline="central">proc · fs · net</text><text class="ts" x="${x + 75}" y="166" text-anchor="middle" dominant-baseline="central">+ session</text><text class="ts" x="${x + 75}" y="183" text-anchor="middle" dominant-baseline="central">${sub}</text></g>`;
	const pieces: Piece[] = [
		{
			key: "orch",
			el: `<g class="c-purple"><rect x="180" y="20" width="320" height="44" rx="10" stroke-width="0.5"/><text class="th" x="340" y="36" text-anchor="middle" dominant-baseline="central">orchestration</text><text class="ts" x="340" y="52" text-anchor="middle" dominant-baseline="central">session router</text></g>`,
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
			el: `<rect class="c-gray-fill" x="60" y="224" width="560" height="42" rx="8" stroke-width="0.5"/><text class="th" x="340" y="240" text-anchor="middle" dominant-baseline="central">shared kernel surface</text><text class="ts" x="340" y="256" text-anchor="middle" dominant-baseline="central">one proc · fs · net boundary</text>`,
		},
	];
	return { vb: "40 0 600 282", pieces };
}

export function SyscallDiagram() {
	const [step, setStep] = useState(1);

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
			let next = anchors[0].step;
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

	const { vb, pieces } = step <= 5 ? buildSingle(step) : buildFleet();
	const title = TITLES[step] ?? "";

	return (
		<div className="not-prose syscall-diagram">
			<p className="cap">{title}</p>
			<svg viewBox={vb} role="img" aria-label={`Diagram: ${title}`}>
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
					position: sticky;
					top: 5.5rem;
					z-index: 10;
					margin: 2rem 0;
					padding: 0.6rem 0.75rem 0.45rem;
					background: #EFEFEF;
					border: 1px solid rgb(27 25 22 / 0.1);
					border-radius: 0.75rem;
				}
				.syscall-diagram svg {
					display: block;
					width: 100%;
					height: auto;
					max-height: 58vh;
					margin: 0 auto;
				}
				/* Full width on mobile reads well, but on desktop that gets too
				   large, so cap and center it. */
				@media (min-width: 768px) {
					.syscall-diagram svg { max-width: 26rem; }
				}
				.syscall-diagram .box { fill: none; stroke: #1b1916; stroke-opacity: .2; }
				.syscall-diagram .dash { stroke-opacity: .28; }
				.syscall-diagram .th { fill: #1b1916; font: 600 13px Manrope, ui-sans-serif, system-ui, sans-serif; }
				.syscall-diagram .ts { fill: #56524a; font: 500 11px Manrope, ui-sans-serif, system-ui, sans-serif; }
				.syscall-diagram .arr { stroke: #2e4034; stroke-width: 1.3; fill: none; }
				.syscall-diagram .c-teal rect { fill: #2e4034; fill-opacity: .07; stroke: #2e4034; stroke-opacity: .7; }
				.syscall-diagram .c-teal text { fill: #2e4034; }
				.syscall-diagram .c-purple rect { fill: #1b1916; fill-opacity: .05; stroke: #1b1916; stroke-opacity: .3; }
				.syscall-diagram .c-purple text { fill: #1b1916; }
				.syscall-diagram .c-gray rect { fill: #1b1916; fill-opacity: .04; stroke: #1b1916; stroke-opacity: .15; }
				.syscall-diagram .c-gray text { fill: #56524a; }
				.syscall-diagram .c-gray-fill { fill: #1b1916; fill-opacity: .04; stroke: #1b1916; stroke-opacity: .15; }
				.syscall-diagram .cap {
					margin: .1rem 0 .45rem;
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
