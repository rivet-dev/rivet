import type { ReactNode, SVGProps } from "react";

// Poster-style cover art for cookbook entries. Every cover shares one surreal
// dusk-landscape composition: a gradient sky, a haze band, a horizon line over
// a dark sea, a soft reflection column, film grain, and a vignette. Per-cover
// identity comes from the palette and from what hangs in the sky, keyed by the
// entry slug in COVER_ART below. Unknown slugs fall back to a generic floating
// orb scene with a palette picked deterministically from the slug.
//
// `CookbookCoverDefs` must be rendered exactly once on any page that renders
// covers; it holds the shared grain filter, vignette, glints, and edge mask
// that every cover SVG references by id.

export const COVER_SERIF = '"Perfectly Nineties", Georgia, serif';

// Horizon y coordinate in the 500x700 viewBox.
const HZ = 510;

// Deterministic PRNG so server render and hydration always agree.
function mulberry32(a: number) {
	return () => {
		a |= 0;
		a = (a + 0x6d2b79f5) | 0;
		let t = Math.imul(a ^ (a >>> 15), 1 | a);
		t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

const f = (n: number) => Math.round(n * 10) / 10;

function polar(cx: number, cy: number, r: number, deg: number): [number, number] {
	const a = (deg * Math.PI) / 180;
	return [f(cx + r * Math.cos(a)), f(cy + r * Math.sin(a))];
}

export function toRoman(n: number): string {
	const table: Array<[number, string]> = [
		[1000, "M"],
		[900, "CM"],
		[500, "D"],
		[400, "CD"],
		[100, "C"],
		[90, "XC"],
		[50, "L"],
		[40, "XL"],
		[10, "X"],
		[9, "IX"],
		[5, "V"],
		[4, "IV"],
		[1, "I"],
	];
	let out = "";
	for (const [value, symbol] of table) {
		while (n >= value) {
			out += symbol;
			n -= value;
		}
	}
	return out;
}

type Stop = [number, string, number?];

function GradStops({ stops }: { stops: Stop[] }) {
	return (
		<>
			{stops.map(([offset, color, opacity], i) => (
				<stop key={i} offset={offset} stopColor={color} stopOpacity={opacity} />
			))}
		</>
	);
}

function LGrad({
	id,
	stops,
	...rest
}: { id: string; stops: Stop[] } & SVGProps<SVGLinearGradientElement>) {
	return (
		<linearGradient id={id} x1="0" y1="0" x2="0" y2="1" {...rest}>
			<GradStops stops={stops} />
		</linearGradient>
	);
}

function RGrad({
	id,
	stops,
	...rest
}: { id: string; stops: Stop[] } & SVGProps<SVGRadialGradientElement>) {
	return (
		<radialGradient id={id} {...rest}>
			<GradStops stops={stops} />
		</radialGradient>
	);
}

function HazeDef({ id, color, opacity }: { id: string; color: string; opacity: number }) {
	return (
		<LGrad
			id={id}
			stops={[
				[0, color, 0],
				[0.55, color, opacity],
				[1, color, 0],
			]}
		/>
	);
}

const Haze = ({ id }: { id: string }) => (
	<rect x="0" y="430" width="500" height="80" fill={`url(#${id})`} />
);

const Sea = ({ id }: { id: string }) => (
	<rect x="0" y={HZ} width="500" height="190" fill={`url(#${id})`} />
);

const HorizonLine = ({ color, opacity }: { color: string; opacity: number }) => (
	<line x1="0" y1={HZ} x2="500" y2={HZ} stroke={color} strokeOpacity={opacity} />
);

const Glints = ({ color }: { color: string }) => (
	<g>
		<use href="#cover-glints" color={color} />
	</g>
);

function Figure({ x, h = 14, w = 3 }: { x: number; h?: number; w?: number }) {
	const y = HZ - h;
	return (
		<>
			<rect x={f(x - w / 2)} y={y} width={w} height={h} rx={w / 2} fill="#020203" />
			<circle cx={x} cy={f(y - 1.6)} r={f(w * 0.78)} fill="#020203" />
		</>
	);
}

function Stone({ x, h = 12, w = 4 }: { x: number; h?: number; w?: number }) {
	return (
		<path
			d={`M${f(x - w / 2)} ${HZ} L${f(x - w / 2 + 0.6)} ${f(HZ - h)} L${f(x + w / 2 - 0.6)} ${f(HZ - h + 1)} L${f(x + w / 2)} ${HZ} Z`}
			fill="#020203"
		/>
	);
}

function Stars({
	seed,
	color,
	n,
	yMin = 36,
	yMax = 360,
}: {
	seed: number;
	color: string;
	n: number;
	yMin?: number;
	yMax?: number;
}) {
	const r = mulberry32(seed);
	const items: ReactNode[] = [];
	for (let i = 0; i < n; i++) {
		const x = f(18 + r() * 464);
		const y = f(yMin + r() * (yMax - yMin));
		const rad = f(0.6 + r() * 1.0);
		const o = f(0.08 + r() * 0.22);
		items.push(<circle key={i} cx={x} cy={y} r={rad} fill={color} opacity={o} />);
	}
	return <>{items}</>;
}

const GLINT_ROWS: Array<[number, number, number, number]> = [
	[524, 178, 322, 0.5],
	[531, 118, 252, 0.42],
	[540, 258, 402, 0.36],
	[551, 58, 192, 0.3],
	[561, 298, 458, 0.27],
	[574, 148, 332, 0.23],
	[589, 38, 152, 0.19],
	[603, 228, 412, 0.16],
	[621, 88, 222, 0.13],
	[639, 278, 432, 0.1],
	[661, 138, 302, 0.08],
	[680, 230, 360, 0.06],
];

// Shared defs referenced by every cover SVG. Render once per page.
export function CookbookCoverDefs() {
	return (
		<svg width="0" height="0" className="absolute" aria-hidden="true">
			<defs>
				<filter id="cover-grain" x="-5%" y="-5%" width="110%" height="110%">
					<feTurbulence
						type="fractalNoise"
						baseFrequency="0.8"
						numOctaves="2"
						stitchTiles="stitch"
					/>
					<feColorMatrix type="saturate" values="0" />
					<feComponentTransfer>
						<feFuncA type="linear" slope="0" intercept="1" />
					</feComponentTransfer>
				</filter>
				<radialGradient id="cover-vig" cx="0.5" cy="0.44" r="0.72">
					<stop offset="0.55" stopColor="#000" stopOpacity="0" />
					<stop offset="1" stopColor="#000" stopOpacity="0.58" />
				</radialGradient>
				<g id="cover-glints">
					{GLINT_ROWS.map(([y, x1, x2, o], i) => (
						<line
							key={i}
							x1={x1}
							y1={y}
							x2={x2}
							y2={y}
							stroke="currentColor"
							strokeOpacity={o}
						/>
					))}
				</g>
				<linearGradient id="cover-softxg" x1="0" y1="0" x2="1" y2="0">
					<stop offset="0" stopColor="#000" />
					<stop offset="0.24" stopColor="#fff" />
					<stop offset="0.76" stopColor="#fff" />
					<stop offset="1" stopColor="#000" />
				</linearGradient>
				<mask id="cover-softx" maskContentUnits="objectBoundingBox">
					<rect width="1" height="1" fill="url(#cover-softxg)" />
				</mask>
			</defs>
		</svg>
	);
}

// A memory orb raining light streams over a violet sea.
function AiAgentArt() {
	const r = mulberry32(11);
	const streams: ReactNode[] = [];
	for (let i = 0; i < 16; i++) {
		const x = f(168 + r() * 164);
		const y1 = f(334 + r() * 24);
		const y2 = f(y1 + 60 + r() * 120);
		streams.push(
			<line
				key={i}
				x1={x}
				y1={y1}
				x2={x}
				y2={Math.min(y2, 504)}
				stroke="url(#c1fall)"
				strokeOpacity={f(0.35 + r() * 0.5)}
			/>,
		);
	}
	return (
		<>
			<defs>
				<LGrad
					id="c1sky"
					stops={[
						[0, "#050511"],
						[0.42, "#120f2c"],
						[0.74, "#2c2154"],
						[1, "#4c3a72"],
					]}
				/>
				<RGrad
					id="c1orb"
					stops={[
						[0, "#f4eeff"],
						[0.5, "#cbb6ef"],
						[1, "#7f64b5"],
					]}
				/>
				<RGrad
					id="c1glow"
					stops={[
						[0, "#ab8fe8", 0.5],
						[1, "#ab8fe8", 0],
					]}
				/>
				<LGrad
					id="c1fall"
					stops={[
						[0, "#d4c6f6", 0.55],
						[1, "#d4c6f6", 0],
					]}
					gradientUnits="userSpaceOnUse"
					x1="0"
					y1="330"
					x2="0"
					y2="505"
				/>
				<HazeDef id="c1haze" color="#8d72c4" opacity={0.16} />
				<LGrad
					id="c1sea"
					stops={[
						[0, "#241a3e"],
						[0.5, "#0c0918"],
						[1, "#020204"],
					]}
				/>
				<LGrad
					id="c1ref"
					stops={[
						[0, "#8d72c4", 0.3],
						[1, "#8d72c4", 0],
					]}
				/>
			</defs>
			<rect width="500" height="700" fill="url(#c1sky)" />
			<Stars seed={101} color="#cfc2f0" n={14} />
			<Haze id="c1haze" />
			<circle cx="250" cy="268" r="150" fill="url(#c1glow)" />
			<circle cx="250" cy="268" r="92" fill="none" stroke="#d9cbf8" strokeOpacity="0.22" />
			<circle cx="250" cy="268" r="118" fill="none" stroke="#d9cbf8" strokeOpacity="0.13" />
			<circle cx="250" cy="268" r="146" fill="none" stroke="#d9cbf8" strokeOpacity="0.07" />
			<circle cx="250" cy="268" r="62" fill="url(#c1orb)" />
			{streams}
			<Sea id="c1sea" />
			<rect x="205" y={HZ} width="90" height="150" fill="url(#c1ref)" mask="url(#cover-softx)" />
			<Glints color="#5d4f86" />
			<HorizonLine color="#b9a4e6" opacity={0.35} />
			<Figure x={172} />
		</>
	);
}

// Slabs ascend in an arc from a grounded monolith toward the upper right.
function WorkspacesArt() {
	const slabs: Array<[number, number, number, number]> = [
		[126, 338, 68, 94],
		[244, 286, 54, 76],
		[312, 224, 42, 60],
		[402, 248, 30, 44],
		[428, 168, 20, 30],
		[180, 420, 28, 40],
	];
	return (
		<>
			<defs>
				<LGrad
					id="c2sky"
					stops={[
						[0, "#030605"],
						[0.45, "#0e1916"],
						[0.78, "#233a32"],
						[1, "#3a5a4a"],
					]}
				/>
				<LGrad
					id="c2slabglow"
					stops={[
						[0, "#bfe6d2", 0],
						[1, "#bfe6d2", 0.26],
					]}
				/>
				<RGrad
					id="c2under"
					stops={[
						[0, "#9fd6bd", 0.2],
						[1, "#9fd6bd", 0],
					]}
				/>
				<RGrad
					id="c2glow"
					stops={[
						[0, "#7eae9a", 0.22],
						[1, "#7eae9a", 0],
					]}
				/>
				<HazeDef id="c2haze" color="#6f9b8c" opacity={0.16} />
				<LGrad
					id="c2sea"
					stops={[
						[0, "#1d322a"],
						[0.5, "#0a1410"],
						[1, "#020403"],
					]}
				/>
				<LGrad
					id="c2ref"
					stops={[
						[0, "#5e8d7c", 0.14],
						[1, "#5e8d7c", 0],
					]}
				/>
			</defs>
			<rect width="500" height="700" fill="url(#c2sky)" />
			<Stars seed={202} color="#bfe0d0" n={10} />
			<Haze id="c2haze" />
			<circle cx="260" cy="320" r="210" fill="url(#c2glow)" />
			{slabs.map(([x, y, w, h], i) => {
				const cx = f(x + w / 2);
				return (
					<g key={i}>
						<ellipse
							cx={cx}
							cy={f(y + h + 9)}
							rx={f(w * 0.72)}
							ry="6.5"
							fill="url(#c2under)"
						/>
						<rect
							x={x}
							y={y}
							width={w}
							height={h}
							fill="#071009"
							stroke="#9fc6b4"
							strokeOpacity="0.34"
						/>
						<rect
							x={x + 1}
							y={f(y + h - h * 0.4)}
							width={w - 2}
							height={f(h * 0.4 - 1)}
							fill="url(#c2slabglow)"
						/>
						<line
							x1={x}
							y1={f(y + h - 0.5)}
							x2={x + w}
							y2={f(y + h - 0.5)}
							stroke="#cfeede"
							strokeOpacity="0.6"
						/>
						<circle
							cx={f(x + w * 0.74)}
							cy={f(y + h * 0.2)}
							r={f(Math.max(1.5, w * 0.04))}
							fill="#d8f2e4"
							opacity="0.9"
						/>
					</g>
				);
			})}
			<rect x="92" y="444" width="16" height="66" fill="#040806" />
			<line x1="92.5" y1="444" x2="92.5" y2="510" stroke="#a9cfbd" strokeOpacity="0.4" />
			<line x1="92" y1="444.5" x2="108" y2="444.5" stroke="#cfeede" strokeOpacity="0.5" />
			<Sea id="c2sea" />
			<rect x="80" y={HZ} width="340" height="140" fill="url(#c2ref)" mask="url(#cover-softx)" />
			<Glints color="#3e5f55" />
			<HorizonLine color="#9fc6b4" opacity={0.3} />
			<Stone x={440} h={10} w={3.5} />
		</>
	);
}

// Broadcast ripple rings with member dots over a teal sea.
function ChatRoomArt() {
	const cx = 250;
	const cy = 396;
	const rings: Array<[number, number]> = [
		[38, 0.5],
		[76, 0.33],
		[122, 0.2],
		[176, 0.12],
		[240, 0.07],
	];
	const dots: Array<[number, number, number]> = [
		[76, 205, 3.4],
		[76, 332, 3],
		[122, 158, 3],
		[122, 22, 3.6],
		[122, 262, 2.6],
		[38, 120, 2.4],
		[176, 196, 2.6],
		[176, 348, 2.2],
	];
	const ripples: Array<[number, number, number]> = [
		[46, 7, 0.28],
		[90, 13, 0.16],
		[150, 22, 0.09],
	];
	return (
		<>
			<defs>
				<LGrad
					id="c3sky"
					stops={[
						[0, "#02070a"],
						[0.45, "#062028"],
						[0.78, "#0d4049"],
						[1, "#136066"],
					]}
				/>
				<RGrad
					id="c3glow"
					stops={[
						[0, "#6fe0d4", 0.5],
						[1, "#6fe0d4", 0],
					]}
				/>
				<HazeDef id="c3haze" color="#4fb3a8" opacity={0.13} />
				<LGrad
					id="c3sea"
					stops={[
						[0, "#0b3a40"],
						[0.45, "#062023"],
						[1, "#010404"],
					]}
				/>
				<LGrad
					id="c3ref"
					stops={[
						[0, "#4fb3a8", 0.25],
						[1, "#4fb3a8", 0],
					]}
				/>
			</defs>
			<rect width="500" height="700" fill="url(#c3sky)" />
			<Stars seed={303} color="#bdebe4" n={8} />
			<Haze id="c3haze" />
			{rings.map(([r, o], i) => (
				<circle key={i} cx={cx} cy={cy} r={r} fill="none" stroke="#9fe8df" strokeOpacity={o} />
			))}
			<circle cx={cx} cy={cy} r="64" fill="url(#c3glow)" />
			<circle cx={cx} cy={cy} r="5" fill="#d8f5f2" />
			{dots.map(([rr, a, rad], i) => {
				const [x, y] = polar(cx, cy, rr, a);
				return <circle key={i} cx={x} cy={y} r={rad} fill="#cdf2ec" opacity="0.9" />;
			})}
			<Sea id="c3sea" />
			<rect x="218" y={HZ} width="64" height="120" fill="url(#c3ref)" mask="url(#cover-softx)" />
			{ripples.map(([rx, ry, o], i) => (
				<ellipse
					key={i}
					cx="250"
					cy="548"
					rx={rx}
					ry={ry}
					fill="none"
					stroke="#58c4b8"
					strokeOpacity={o}
				/>
			))}
			<Glints color="#2a6a64" />
			<HorizonLine color="#7fd8cd" opacity={0.3} />
			<Stone x={142} h={9} w={3} />
			<Stone x={150} h={13} w={4} />
			<Stone x={159} h={10} w={3} />
		</>
	);
}

// Rays from every collaborator converge on a single glowing caret.
function CollabEditorArt() {
	const pt: [number, number] = [250, 504];
	const edges: Array<[number, number]> = [];
	for (let x = 16; x <= 484; x += 52) edges.push([x, 0]);
	for (let y = 120; y <= 460; y += 85) {
		edges.push([0, y]);
		edges.push([500, y]);
	}
	return (
		<>
			<defs>
				<LGrad
					id="c4sky"
					stops={[
						[0, "#060302"],
						[0.45, "#170a06"],
						[0.78, "#33110a"],
						[1, "#531d0b"],
					]}
				/>
				<RGrad
					id="c4ray"
					stops={[
						[0, "#ffd2b3", 0.85],
						[0.16, "#ff7a3a", 0.3],
						[0.5, "#ff5a18", 0.08],
						[1, "#ff5a18", 0],
					]}
					gradientUnits="userSpaceOnUse"
					cx="250"
					cy="504"
					r="520"
				/>
				<RGrad
					id="c4glow1"
					stops={[
						[0, "#FF4500", 0.5],
						[1, "#FF4500", 0],
					]}
				/>
				<RGrad
					id="c4glow2"
					stops={[
						[0, "#ff6a22", 0.6],
						[1, "#ff6a22", 0],
					]}
				/>
				<HazeDef id="c4haze" color="#c9521f" opacity={0.14} />
				<LGrad
					id="c4sea"
					stops={[
						[0, "#2b1009"],
						[0.5, "#100604"],
						[1, "#020101"],
					]}
				/>
				<LGrad
					id="c4ref"
					stops={[
						[0, "#FF4500", 0.32],
						[1, "#FF4500", 0],
					]}
				/>
			</defs>
			<rect width="500" height="700" fill="url(#c4sky)" />
			<Haze id="c4haze" />
			{edges.map(([x, y], i) => (
				<line key={i} x1={x} y1={y} x2={pt[0]} y2={pt[1]} stroke="url(#c4ray)" />
			))}
			<circle cx="250" cy="504" r="140" fill="url(#c4glow1)" />
			<circle cx="250" cy="504" r="56" fill="url(#c4glow2)" />
			<circle cx="250" cy="504" r="5.5" fill="#ffe6d6" />
			<line x1="250" y1="438" x2="250" y2="500" stroke="#ffe9da" strokeWidth="2" strokeOpacity="0.95" />
			<line x1="244" y1="438" x2="256" y2="438" stroke="#ffe9da" strokeOpacity="0.7" />
			<Sea id="c4sea" />
			<rect x="186" y={HZ} width="128" height="160" fill="url(#c4ref)" mask="url(#cover-softx)" />
			<line x1="216" y1="523" x2="288" y2="523" stroke="#ff6a2a" strokeOpacity="0.5" />
			<line x1="196" y1="532" x2="262" y2="532" stroke="#ff6a2a" strokeOpacity="0.32" />
			<line x1="232" y1="544" x2="312" y2="544" stroke="#ff6a2a" strokeOpacity="0.2" />
			<Glints color="#4f2517" />
			<HorizonLine color="#ff9a66" opacity={0.35} />
			<Figure x={118} h={13} />
			<Figure x={376} h={12} />
		</>
	);
}

// A half-risen dial sun with tick marks burning on the horizon.
function CronArt() {
	const cx = 250;
	const cy = 512;
	const ticks: ReactNode[] = [];
	for (let a = 184; a <= 356; a += 4) {
		const major = a % 20 === 0;
		const r1 = major ? 108 : 114;
		const r2 = major ? 130 : 123;
		const [x1, y1] = polar(cx, cy, r1, a);
		const [x2, y2] = polar(cx, cy, r2, a);
		ticks.push(
			<line
				key={a}
				x1={x1}
				y1={y1}
				x2={x2}
				y2={y2}
				stroke="#ffb88a"
				strokeOpacity={major ? 0.8 : 0.42}
				strokeWidth={major ? 1.6 : 1}
			/>,
		);
	}
	const [hx1, hy1] = polar(cx, cy, 104, 305);
	const [hx2, hy2] = polar(cx, cy, 142, 305);
	const stoneXs = [55, 120, 185, 250, 315, 380, 445];
	return (
		<>
			<defs>
				<LGrad
					id="c5sky"
					stops={[
						[0, "#090301"],
						[0.4, "#1f0a03"],
						[0.72, "#3f1605"],
						[1, "#6b2708"],
					]}
				/>
				<RGrad
					id="c5glow"
					stops={[
						[0, "#ff5a14", 0.5],
						[0.5, "#ff5a14", 0.16],
						[1, "#ff5a14", 0],
					]}
				/>
				<RGrad
					id="c5sun"
					stops={[
						[0, "#ffc089"],
						[0.35, "#ff7a2e"],
						[0.7, "#FF4500"],
						[1, "#d83800"],
					]}
				/>
				<HazeDef id="c5haze" color="#ff7a30" opacity={0.15} />
				<LGrad
					id="c5sea"
					stops={[
						[0, "#3a1404"],
						[0.5, "#150703"],
						[1, "#030101"],
					]}
				/>
				<LGrad
					id="c5ref"
					stops={[
						[0, "#FF4500", 0.38],
						[1, "#FF4500", 0],
					]}
				/>
			</defs>
			<rect width="500" height="700" fill="url(#c5sky)" />
			<Haze id="c5haze" />
			<circle cx={cx} cy={cy} r="230" fill="url(#c5glow)" />
			{ticks}
			<circle cx={cx} cy={cy} r="90" fill="url(#c5sun)" />
			<line x1={hx1} y1={hy1} x2={hx2} y2={hy2} stroke="#ffe2c8" strokeWidth="2.2" strokeOpacity="0.95" />
			<Sea id="c5sea" />
			<rect x="178" y={HZ} width="144" height="170" fill="url(#c5ref)" mask="url(#cover-softx)" />
			<line x1="206" y1="524" x2="298" y2="524" stroke="#ff7a30" strokeOpacity="0.6" />
			<line x1="186" y1="534" x2="266" y2="534" stroke="#ff7a30" strokeOpacity="0.42" />
			<line x1="224" y1="548" x2="316" y2="548" stroke="#ff7a30" strokeOpacity="0.28" />
			<line x1="204" y1="566" x2="284" y2="566" stroke="#ff7a30" strokeOpacity="0.16" />
			<Glints color="#6e3413" />
			<HorizonLine color="#ffb27d" opacity={0.4} />
			{stoneXs.map((x, i) => (
				<Stone key={x} x={x} h={9 + (i % 3) * 2.5} w={3.6} />
			))}
		</>
	);
}

// A constellation of dart cursors with trails in a slate blue sky.
function CursorsArt() {
	const darts: Array<[number, number, number, number, number]> = [
		[140, 252, -32, 1.1, 0.9],
		[305, 224, 42, 0.9, 0.72],
		[228, 332, 12, 1.35, 0.95],
		[352, 318, -16, 0.8, 0.6],
		[96, 350, 56, 0.7, 0.5],
		[262, 410, -52, 0.9, 0.7],
		[180, 434, 26, 0.62, 0.45],
		[398, 402, 16, 0.62, 0.4],
		[320, 462, -30, 0.5, 0.35],
	];
	const links: Array<[number, number, number, number]> = [
		[140, 252, 305, 224],
		[228, 332, 140, 252],
		[228, 332, 352, 318],
		[262, 410, 228, 332],
	];
	return (
		<>
			<defs>
				<path id="c6d" d="M0 -8 L5 4 L0 8 L-5 4 Z" fill="#dbe6f7" />
				<LGrad
					id="c6sky"
					stops={[
						[0, "#02060e"],
						[0.45, "#091324"],
						[0.78, "#142b46"],
						[1, "#1d3f63"],
					]}
				/>
				<HazeDef id="c6haze" color="#6f93c4" opacity={0.14} />
				<LGrad
					id="c6sea"
					stops={[
						[0, "#15293f"],
						[0.5, "#081220"],
						[1, "#010204"],
					]}
				/>
				<LGrad
					id="c6ref"
					stops={[
						[0, "#4c7099", 0.12],
						[1, "#4c7099", 0],
					]}
				/>
			</defs>
			<rect width="500" height="700" fill="url(#c6sky)" />
			<Stars seed={606} color="#cddcf2" n={14} />
			<Haze id="c6haze" />
			<circle cx="74" cy="272" r="15" fill="#cfdcef" opacity="0.13" />
			<circle cx="74" cy="272" r="15" fill="none" stroke="#cfdcef" strokeOpacity="0.26" />
			{links.map(([x1, y1, x2, y2], i) => (
				<line key={i} x1={x1} y1={y1} x2={x2} y2={y2} stroke="#8fb0da" strokeOpacity="0.17" />
			))}
			{darts.map(([x, y, rot, s, o], i) => {
				const a = ((rot + 90) * Math.PI) / 180;
				const len = 26 + s * 30;
				const tx1 = f(x + Math.cos(a) * 9 * s);
				const ty1 = f(y + Math.sin(a) * 9 * s);
				const tx2 = f(x + Math.cos(a) * (9 * s + len));
				const ty2 = f(y + Math.sin(a) * (9 * s + len));
				return (
					<line
						key={i}
						x1={tx1}
						y1={ty1}
						x2={tx2}
						y2={ty2}
						stroke="#b9cdf0"
						strokeOpacity={f(o * 0.35)}
					/>
				);
			})}
			{darts.map(([x, y, rot, s, o], i) => (
				<use
					key={i}
					href="#c6d"
					transform={`translate(${x} ${y}) rotate(${rot}) scale(${s})`}
					opacity={o}
				/>
			))}
			<Sea id="c6sea" />
			<rect x="80" y={HZ} width="340" height="130" fill="url(#c6ref)" mask="url(#cover-softx)" />
			<Glints color="#31506f" />
			<HorizonLine color="#9db8dd" opacity={0.3} />
			<Figure x={296} h={13} />
		</>
	);
}

// A ringed sphere with a moon over a perspective grid plain.
function GameArt() {
	const horiz: Array<[number, number]> = [
		[514, 0.35],
		[521, 0.3],
		[531, 0.26],
		[545, 0.22],
		[564, 0.18],
		[590, 0.14],
		[624, 0.11],
		[668, 0.08],
	];
	const radial = [-140, -40, 40, 120, 200, 250, 300, 380, 460, 540, 640];
	return (
		<>
			<defs>
				<LGrad
					id="c7sky"
					stops={[
						[0, "#070311"],
						[0.45, "#150b26"],
						[0.78, "#2a1542"],
						[1, "#3d1f58"],
					]}
				/>
				<RGrad
					id="c7sph"
					stops={[
						[0, "#efe6fa"],
						[0.45, "#b79ddc"],
						[1, "#6a4d96"],
					]}
				/>
				<RGrad
					id="c7glow"
					stops={[
						[0, "#9b78cf", 0.32],
						[1, "#9b78cf", 0],
					]}
				/>
				<LGrad
					id="c7gnd"
					stops={[
						[0, "#1c0f2e"],
						[0.5, "#0d0718"],
						[1, "#020104"],
					]}
				/>
				<HazeDef id="c7haze" color="#9b78cf" opacity={0.15} />
			</defs>
			<rect width="500" height="700" fill="url(#c7sky)" />
			<Stars seed={707} color="#d8c8ef" n={12} />
			<Haze id="c7haze" />
			<circle cx="250" cy="308" r="110" fill="url(#c7glow)" />
			<circle cx="250" cy="308" r="42" fill="url(#c7sph)" />
			<g transform="rotate(-16 250 308)">
				<ellipse
					cx="250"
					cy="308"
					rx="84"
					ry="22"
					fill="none"
					stroke="#d4bff0"
					strokeOpacity="0.5"
					strokeWidth="1.2"
				/>
				<ellipse cx="250" cy="308" rx="118" ry="32" fill="none" stroke="#d4bff0" strokeOpacity="0.2" />
				<circle cx="153.4" cy="289.7" r="7" fill="#d4bff0" opacity="0.25" />
				<circle cx="153.4" cy="289.7" r="3.4" fill="#f0e6fb" opacity="0.95" />
			</g>
			<rect x="0" y={HZ} width="500" height="190" fill="url(#c7gnd)" />
			{horiz.map(([y, o], i) => (
				<line key={i} x1="0" y1={y} x2="500" y2={y} stroke="#8a63bd" strokeOpacity={o} />
			))}
			{radial.map((x, i) => (
				<line key={i} x1="250" y1={HZ} x2={x} y2="700" stroke="#8a63bd" strokeOpacity="0.12" />
			))}
			<HorizonLine color="#b993e0" opacity={0.35} />
			<Stone x={150} h={20} w={5} />
			<Stone x={342} h={16} w={4} />
		</>
	);
}

// Five identical sealed vaults, each under its own beam of light.
function TenantsArt() {
	const centers = [90, 170, 250, 330, 410];
	return (
		<>
			<defs>
				<LGrad
					id="c8sky"
					stops={[
						[0, "#020407"],
						[0.5, "#0a121b"],
						[0.82, "#16222e"],
						[1, "#243747"],
					]}
				/>
				<LGrad
					id="c8band"
					stops={[
						[0, "#8fb4d0", 0],
						[0.7, "#8fb4d0", 0.22],
						[1, "#8fb4d0", 0.3],
					]}
				/>
				<LGrad
					id="c8beam"
					stops={[
						[0, "#a8c8e0", 0],
						[1, "#a8c8e0", 0.12],
					]}
				/>
				<HazeDef id="c8haze" color="#7fa2bd" opacity={0.13} />
				<LGrad
					id="c8sea"
					stops={[
						[0, "#16242f"],
						[0.5, "#08111a"],
						[1, "#010203"],
					]}
				/>
				<LGrad
					id="c8ref"
					stops={[
						[0, "#5e7c93", 0.18],
						[1, "#5e7c93", 0],
					]}
				/>
			</defs>
			<rect width="500" height="700" fill="url(#c8sky)" />
			<Stars seed={808} color="#c8d8e8" n={14} />
			<Haze id="c8haze" />
			<rect x="0" y="436" width="500" height="74" fill="url(#c8band)" />
			{centers.map((cx) => (
				<rect
					key={cx}
					x={cx - 13}
					y="206"
					width="26"
					height="184"
					fill="url(#c8beam)"
					mask="url(#cover-softx)"
				/>
			))}
			{centers.map((cx) => {
				const x = cx - 16;
				return (
					<g key={cx}>
						<rect x={x} y="388" width="32" height="122" fill="#03060a" />
						<line x1={x + 0.5} y1="388" x2={x + 0.5} y2="510" stroke="#b9d2e4" strokeOpacity="0.4" />
						<line x1={x} y1="388.5" x2={x + 32} y2="388.5" stroke="#b9d2e4" strokeOpacity="0.55" />
						<line x1={x + 4} y1="428" x2={x + 28} y2="428" stroke="#b9d2e4" strokeOpacity="0.16" />
					</g>
				);
			})}
			<Sea id="c8sea" />
			{centers.map((cx) => (
				<rect
					key={cx}
					x={cx - 14}
					y={HZ}
					width="28"
					height="80"
					fill="url(#c8ref)"
					mask="url(#cover-softx)"
				/>
			))}
			<Glints color="#2c4254" />
			<HorizonLine color="#9fbdd4" opacity={0.35} />
		</>
	);
}

const FALLBACK_PALETTES = [
	{
		sky: [
			[0, "#050b11"],
			[0.45, "#0c1c28"],
			[0.78, "#173448"],
			[1, "#22506b"],
		] as Stop[],
		body: "#9cc4de",
		glow: "#7fb2d4",
		haze: "#6f9cc4",
		sea: [
			[0, "#13283a"],
			[0.5, "#071420"],
			[1, "#010204"],
		] as Stop[],
		glint: "#2e4f6b",
		horizon: "#9dc1dd",
	},
	{
		sky: [
			[0, "#0a0508"],
			[0.45, "#1d0d18"],
			[0.78, "#3a1a30"],
			[1, "#5b2a48"],
		] as Stop[],
		body: "#e0a8c8",
		glow: "#c486ab",
		haze: "#b07697",
		sea: [
			[0, "#311527"],
			[0.5, "#130810"],
			[1, "#030102"],
		] as Stop[],
		glint: "#5e3049",
		horizon: "#dba4c4",
	},
	{
		sky: [
			[0, "#0b0903"],
			[0.45, "#1f1a08"],
			[0.78, "#3d3411"],
			[1, "#5e511c"],
		] as Stop[],
		body: "#e6d49a",
		glow: "#cdb878",
		haze: "#b3a065",
		sea: [
			[0, "#2e2810"],
			[0.5, "#120f06"],
			[1, "#020201"],
		] as Stop[],
		glint: "#5c5126",
		horizon: "#dccb93",
	},
	{
		sky: [
			[0, "#040810"],
			[0.45, "#0d1430"],
			[0.78, "#1b2756"],
			[1, "#2c3c7a"],
		] as Stop[],
		body: "#aebcf0",
		glow: "#8d9fdd",
		haze: "#7c8cc8",
		sea: [
			[0, "#18204a"],
			[0.5, "#0a0d20"],
			[1, "#020205"],
		] as Stop[],
		glint: "#36406e",
		horizon: "#a9b8ea",
	},
];

function hashSlug(slug: string): number {
	let h = 0;
	for (let i = 0; i < slug.length; i++) {
		h = (h * 31 + slug.charCodeAt(i)) | 0;
	}
	return Math.abs(h);
}

// Generic floating orb scene for entries without bespoke artwork.
function FallbackArt({ slug }: { slug: string }) {
	const hash = hashSlug(slug);
	const palette = FALLBACK_PALETTES[hash % FALLBACK_PALETTES.length];
	const id = (part: string) => `fb-${slug}-${part}`;
	return (
		<>
			<defs>
				<LGrad id={id("sky")} stops={palette.sky} />
				<RGrad
					id={id("orb")}
					stops={[
						[0, "#ffffff"],
						[0.5, palette.body],
						[1, palette.glow],
					]}
				/>
				<RGrad
					id={id("glow")}
					stops={[
						[0, palette.glow, 0.45],
						[1, palette.glow, 0],
					]}
				/>
				<HazeDef id={id("haze")} color={palette.haze} opacity={0.15} />
				<LGrad id={id("sea")} stops={palette.sea} />
				<LGrad
					id={id("ref")}
					stops={[
						[0, palette.haze, 0.26],
						[1, palette.haze, 0],
					]}
				/>
			</defs>
			<rect width="500" height="700" fill={`url(#${id("sky")})`} />
			<Stars seed={hash % 9973} color={palette.body} n={12} />
			<Haze id={id("haze")} />
			<circle cx="250" cy="284" r="140" fill={`url(#${id("glow")})`} />
			<circle cx="250" cy="284" r="96" fill="none" stroke={palette.body} strokeOpacity="0.2" />
			<circle cx="250" cy="284" r="128" fill="none" stroke={palette.body} strokeOpacity="0.1" />
			<circle cx="250" cy="284" r="54" fill={`url(#${id("orb")})`} />
			<Sea id={id("sea")} />
			<rect
				x="200"
				y={HZ}
				width="100"
				height="140"
				fill={`url(#${id("ref")})`}
				mask="url(#cover-softx)"
			/>
			<Glints color={palette.glint} />
			<HorizonLine color={palette.horizon} opacity={0.32} />
			<Figure x={(hash % 280) + 110} h={13} />
		</>
	);
}

const COVER_ART: Record<string, () => ReactNode> = {
	"ai-agent": AiAgentArt,
	"ai-agent-workspace": WorkspacesArt,
	"chat-room": ChatRoomArt,
	"collaborative-text-editor": CollabEditorArt,
	"cron-jobs": CronArt,
	"live-cursors": CursorsArt,
	"multiplayer-game": GameArt,
	"per-tenant-database": TenantsArt,
};

function RivetMark({ className }: { className?: string }) {
	return (
		<svg
			viewBox="0 0 128 128"
			fill="none"
			xmlns="http://www.w3.org/2000/svg"
			aria-hidden="true"
			className={className}
		>
			<rect x="18.25" y="18.25" width="91.5" height="91.5" rx="25.75" stroke="#F0F0F0" strokeWidth="8.5" />
			<path
				fillRule="evenodd"
				clipRule="evenodd"
				d="M57.694 43.098c0-.622-.505-1.126-1.127-1.126h-8.444a5.114 5.114 0 0 0-5.112 5.111v33.824a5.114 5.114 0 0 0 5.112 5.112h8.444c.622 0 1.127-.505 1.127-1.127V43.098Zm24.424 27.869c-1.238-2.222-4.047-4.026-6.27-4.026H62.923c-.684 0-.93.555-.549 1.239l7.703 13.822c1.239 2.223 4.048 4.026 6.27 4.026h12.927c.683 0 .93-.555.548-1.239l-7.703-13.822Zm.538-18.718c0-5.672-4.605-10.277-10.277-10.277H63.31a1.21 1.21 0 0 0-1.209 1.209v18.137c0 .667.542 1.209 1.21 1.209h9.068c5.672 0 10.277-4.605 10.277-10.278Z"
				fill="#F0F0F0"
			/>
		</svg>
	);
}

export interface CookbookCoverProps {
	slug: string;
	title: string;
	numeral: string;
}

// Renders the full poster inside a parent that must be `relative`,
// `overflow-hidden`, sized to aspect 5/7, have `container-type: inline-size`,
// and carry the `group` class for the hover treatment.
export function CookbookCover({ slug, title, numeral }: CookbookCoverProps) {
	const Art = COVER_ART[slug];
	const longTitle = title.length > 18;
	return (
		<>
			<svg
				viewBox="0 0 500 700"
				preserveAspectRatio="xMidYMid slice"
				aria-hidden="true"
				className="absolute inset-0 block h-full w-full transition-transform duration-700 group-hover:scale-[1.03]"
			>
				{Art ? <Art /> : <FallbackArt slug={slug} />}
				<rect width="500" height="700" fill="url(#cover-vig)" />
				<rect
					width="500"
					height="700"
					filter="url(#cover-grain)"
					style={{ mixBlendMode: "overlay", opacity: 0.38 }}
				/>
				<rect
					width="500"
					height="700"
					filter="url(#cover-grain)"
					style={{ mixBlendMode: "screen", opacity: 0.05 }}
				/>
			</svg>
			<div className="pointer-events-none absolute inset-x-0 top-0 px-[7cqw] pt-[11cqw] text-center">
				<div aria-hidden="true" className="mb-[3.6cqw] flex items-center justify-center gap-[3cqw]">
					<span className="h-px w-[7cqw] bg-[#ece9e2]/35" />
					<span
						className="text-[3.4cqw] tracking-[0.28em] text-[#ece9e2]/[0.66]"
						style={{ fontFamily: COVER_SERIF }}
					>
						{numeral}
					</span>
					<span className="h-px w-[7cqw] bg-[#ece9e2]/35" />
				</div>
				<h2
					className={`${longTitle ? "text-[6.3cqw]" : "text-[7.4cqw]"} font-normal uppercase leading-[1.24] tracking-[0.13em] text-[#EDEAE3] [text-wrap:balance]`}
					style={{
						fontFamily: COVER_SERIF,
						textShadow: "0 2px 22px rgba(0,0,0,0.55), 0 0 6px rgba(0,0,0,0.3)",
					}}
				>
					{title}
				</h2>
			</div>
			<div className="pointer-events-none absolute inset-x-0 bottom-[6cqw] flex justify-center opacity-[0.72]">
				<RivetMark className="h-auto w-[9cqw]" />
			</div>
		</>
	);
}
