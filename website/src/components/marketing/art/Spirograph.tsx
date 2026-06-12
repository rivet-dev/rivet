// Deterministic hypotrochoid rosette rendered as static SVG. One closed path
// is defined once and stamped as rotated <use> copies; the moiré interference
// comes from the rotation offsets, so the markup stays small. SSR-safe, no
// hooks, no canvas.
//
//   x(t) = (R - r)·cos t + d·cos(((R - r)/r)·t)
//   y(t) = (R - r)·sin t - d·sin(((R - r)/r)·t)
//
// With integer R and r the curve closes after r / gcd(R, r) revolutions.

interface SpirographProps {
	variant?: 'moire' | 'rosette' | 'ring';
	R?: number;
	r?: number;
	d?: number;
	copies?: number;
	size?: number;
	stroke?: string;
	strokeWidth?: number;
	strokeOpacity?: number;
	seed?: number;
	rotate?: boolean;
	className?: string;
}

const PRESETS: Record<string, { R: number; r: number; d: number; copies: number }> = {
	moire: { R: 120, r: 36, d: 84, copies: 48 },
	rosette: { R: 144, r: 60, d: 100, copies: 24 },
	ring: { R: 210, r: 66, d: 90, copies: 3 },
};

const gcd = (a: number, b: number): number => (b ? gcd(b, a % b) : a);

// Small deterministic PRNG so seeded jitter is stable across SSR and client.
const mulberry32 = (seed: number) => {
	let a = seed >>> 0;
	return () => {
		a |= 0;
		a = (a + 0x6d2b79f5) | 0;
		let t = Math.imul(a ^ (a >>> 15), 1 | a);
		t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
};

const hypotrochoidPath = (R: number, r: number, d: number): string => {
	const revolutions = r / gcd(Math.round(R), Math.round(r));
	const maxT = revolutions * 2 * Math.PI;
	const step = 0.05;
	const points: string[] = [];
	for (let t = 0; t <= maxT + step; t += step) {
		const x = (R - r) * Math.cos(t) + d * Math.cos(((R - r) / r) * t);
		const y = (R - r) * Math.sin(t) - d * Math.sin(((R - r) / r) * t);
		points.push(`${x.toFixed(1)},${y.toFixed(1)}`);
	}
	return `M${points.join('L')}Z`;
};

export const Spirograph = ({
	variant = 'ring',
	R,
	r,
	d,
	copies,
	size = 480,
	stroke = '#5A6B52',
	strokeWidth = 0.7,
	strokeOpacity = 0.45,
	seed = 0,
	rotate = false,
	className,
}: SpirographProps) => {
	const preset = PRESETS[variant];
	const pR = R ?? preset.R;
	const pr = r ?? preset.r;
	const pd = d ?? preset.d;
	const pCopies = copies ?? preset.copies;

	// Id derived from the curve parameters: stable across SSR and hydration,
	// and identical instances may safely share a def.
	const id = `spiro-${pR}-${pr}-${pd}-${pCopies}-${stroke.replace(/[^a-zA-Z0-9]/g, '')}`;

	const rand = seed ? mulberry32(seed) : null;
	const paths: string[] = [];
	if (rand) {
		// Seeded mode jitters d per copy for an organic, hand-drawn moiré.
		for (let k = 0; k < pCopies; k++) {
			paths.push(hypotrochoidPath(pR, pr, pd + (rand() - 0.5) * 8));
		}
	} else {
		paths.push(hypotrochoidPath(pR, pr, pd));
	}

	return (
		<svg
			viewBox="-300 -300 600 600"
			width={size}
			height={size}
			aria-hidden="true"
			focusable="false"
			className={`${rotate ? 'animate-spin-slow' : ''} ${className ?? ''}`}
		>
			{rand ? (
				paths.map((path, k) => (
					<path
						key={k}
						d={path}
						fill="none"
						stroke={stroke}
						strokeWidth={strokeWidth}
						strokeOpacity={strokeOpacity}
						transform={`rotate(${((k * 360) / pCopies).toFixed(2)})`}
					/>
				))
			) : (
				<>
					<defs>
						<path
							id={id}
							d={paths[0]}
							fill="none"
							stroke={stroke}
							strokeWidth={strokeWidth}
							strokeOpacity={strokeOpacity}
						/>
					</defs>
					{Array.from({ length: pCopies }, (_, k) => (
						<use key={k} href={`#${id}`} transform={`rotate(${((k * 360) / pCopies).toFixed(2)})`} />
					))}
				</>
			)}
		</svg>
	);
};
