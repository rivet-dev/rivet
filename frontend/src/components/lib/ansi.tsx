import type { ReactNode } from "react";

// ANSI color codes mapped to Tailwind-compatible colors.
// These are tuned for dark backgrounds (the default log view theme).
const ANSI_COLORS: Record<number, string> = {
	// Standard foreground
	30: "hsl(0 0% 30%)", // black -> dark gray
	31: "hsl(0 72% 51%)", // red -> destructive
	32: "hsl(142 71% 45%)", // green
	33: "hsl(48 96% 53%)", // yellow -> warning
	34: "hsl(213 94% 68%)", // blue
	35: "hsl(270 76% 72%)", // magenta
	36: "hsl(186 94% 55%)", // cyan
	37: "hsl(0 0% 80%)", // white -> light gray
	// Bright foreground
	90: "hsl(0 0% 45%)", // bright black -> medium gray
	91: "hsl(0 84% 60%)", // bright red
	92: "hsl(142 71% 60%)", // bright green
	93: "hsl(48 96% 67%)", // bright yellow
	94: "hsl(213 94% 75%)", // bright blue
	95: "hsl(270 76% 82%)", // bright magenta
	96: "hsl(186 94% 70%)", // bright cyan
	97: "hsl(0 0% 97%)", // bright white
};

const ANSI_BG_COLORS: Record<number, string> = {
	40: "hsl(0 0% 10%)",
	41: "hsl(0 72% 20%)",
	42: "hsl(142 71% 15%)",
	43: "hsl(48 96% 15%)",
	44: "hsl(213 94% 20%)",
	45: "hsl(270 76% 20%)",
	46: "hsl(186 94% 15%)",
	47: "hsl(0 0% 60%)",
	100: "hsl(0 0% 20%)",
	101: "hsl(0 84% 30%)",
	102: "hsl(142 71% 25%)",
	103: "hsl(48 96% 25%)",
	104: "hsl(213 94% 30%)",
	105: "hsl(270 76% 30%)",
	106: "hsl(186 94% 25%)",
	107: "hsl(0 0% 80%)",
};

interface SpanStyle {
	color?: string;
	bgColor?: string;
	bold?: boolean;
	dim?: boolean;
	italic?: boolean;
	underline?: boolean;
}

interface Span extends SpanStyle {
	text: string;
	offset: number;
}

function parseAnsi(input: string): Span[] {
	const spans: Span[] = [];
	const regex = /\x1b\[([0-9;]*)m/g;

	let currentStyle: SpanStyle = {};
	let lastIndex = 0;

	for (const match of input.matchAll(regex)) {
		const matchIndex = match.index ?? 0;
		const text = input.slice(lastIndex, matchIndex);
		if (text) {
			spans.push({ text, offset: lastIndex, ...currentStyle });
		}

		const codes = match[1]
			.split(";")
			.map(Number)
			.filter((n) => !Number.isNaN(n));

		let i = 0;
		while (i < codes.length) {
			const code = codes[i];

			if (code === 0) {
				currentStyle = {};
			} else if (code === 1) {
				currentStyle = { ...currentStyle, bold: true };
			} else if (code === 2) {
				currentStyle = { ...currentStyle, dim: true };
			} else if (code === 3) {
				currentStyle = { ...currentStyle, italic: true };
			} else if (code === 4) {
				currentStyle = { ...currentStyle, underline: true };
			} else if (code === 22) {
				const { bold: _b, dim: _d, ...rest } = currentStyle;
				currentStyle = rest;
			} else if (code === 23) {
				const { italic: _i, ...rest } = currentStyle;
				currentStyle = rest;
			} else if (code === 24) {
				const { underline: _u, ...rest } = currentStyle;
				currentStyle = rest;
			} else if (
				code === 38 &&
				codes[i + 1] === 5 &&
				codes[i + 2] !== undefined
			) {
				currentStyle = {
					...currentStyle,
					color: ansi256ToHsl(codes[i + 2]),
				};
				i += 2;
			} else if (
				code === 38 &&
				codes[i + 1] === 2 &&
				codes[i + 4] !== undefined
			) {
				currentStyle = {
					...currentStyle,
					color: `rgb(${codes[i + 2]},${codes[i + 3]},${codes[i + 4]})`,
				};
				i += 4;
			} else if (
				code === 48 &&
				codes[i + 1] === 5 &&
				codes[i + 2] !== undefined
			) {
				currentStyle = {
					...currentStyle,
					bgColor: ansi256ToHsl(codes[i + 2]),
				};
				i += 2;
			} else if (
				code === 48 &&
				codes[i + 1] === 2 &&
				codes[i + 4] !== undefined
			) {
				currentStyle = {
					...currentStyle,
					bgColor: `rgb(${codes[i + 2]},${codes[i + 3]},${codes[i + 4]})`,
				};
				i += 4;
			} else if (ANSI_COLORS[code] !== undefined) {
				currentStyle = { ...currentStyle, color: ANSI_COLORS[code] };
			} else if (ANSI_BG_COLORS[code] !== undefined) {
				currentStyle = {
					...currentStyle,
					bgColor: ANSI_BG_COLORS[code],
				};
			} else if (code === 39) {
				const { color: _c, ...rest } = currentStyle;
				currentStyle = rest;
			} else if (code === 49) {
				const { bgColor: _bg, ...rest } = currentStyle;
				currentStyle = rest;
			}

			i++;
		}

		lastIndex = matchIndex + match[0].length;
	}

	const remaining = input.slice(lastIndex);
	if (remaining) {
		spans.push({ text: remaining, offset: lastIndex, ...currentStyle });
	}

	return spans;
}

function ansi256ToHsl(n: number): string {
	if (n < 16) {
		const systemColors = [
			"hsl(0 0% 0%)",
			"hsl(0 72% 40%)",
			"hsl(142 71% 35%)",
			"hsl(48 96% 40%)",
			"hsl(213 94% 50%)",
			"hsl(270 76% 55%)",
			"hsl(186 94% 40%)",
			"hsl(0 0% 66%)",
			"hsl(0 0% 33%)",
			"hsl(0 84% 60%)",
			"hsl(142 71% 60%)",
			"hsl(48 96% 67%)",
			"hsl(213 94% 75%)",
			"hsl(270 76% 82%)",
			"hsl(186 94% 70%)",
			"hsl(0 0% 97%)",
		];
		return systemColors[n] ?? "hsl(0 0% 97%)";
	}
	if (n < 232) {
		const idx = n - 16;
		const b = idx % 6;
		const g = Math.floor(idx / 6) % 6;
		const r = Math.floor(idx / 36);
		const toChannel = (v: number) => (v === 0 ? 0 : 55 + v * 40);
		return `rgb(${toChannel(r)},${toChannel(g)},${toChannel(b)})`;
	}
	const gray = 8 + (n - 232) * 10;
	return `rgb(${gray},${gray},${gray})`;
}

export function hasAnsi(input: string): boolean {
	return /\x1b/.test(input);
}

/**
 * Renders a string with ANSI escape codes as React spans with inline styles.
 * Falls back to plain text if no ANSI codes are present.
 */
export function AnsiText({ text }: { text: string }): ReactNode {
	if (!hasAnsi(text)) {
		return text;
	}

	const spans = parseAnsi(text);

	return (
		<>
			{spans.map((span) => {
				const hasStyle =
					span.color ||
					span.bgColor ||
					span.bold ||
					span.dim ||
					span.italic ||
					span.underline;

				if (!hasStyle) {
					return <span key={span.offset}>{span.text}</span>;
				}

				return (
					<span
						key={span.offset}
						style={{
							...(span.color && { color: span.color }),
							...(span.bgColor && {
								backgroundColor: span.bgColor,
							}),
							...(span.bold && { fontWeight: "bold" }),
							...(span.dim && { opacity: 0.5 }),
							...(span.italic && { fontStyle: "italic" }),
							...(span.underline && {
								textDecoration: "underline",
							}),
						}}
					>
						{span.text}
					</span>
				);
			})}
		</>
	);
}
