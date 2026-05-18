export type OrgPalette = {
	c1: string;
	c2: string;
	c3: string;
	c4: string;
	accent: string;
};

const PALETTES: OrgPalette[] = [
	// Orange
	{
		c1: "hsl(22 78% 56%)",
		c2: "hsl(8 70% 38%)",
		c3: "hsl(35 82% 62%)",
		c4: "hsl(15 60% 28%)",
		accent: "hsl(20 78% 52%)",
	},
	// Cream
	{
		c1: "hsl(40 40% 98%)",
		c2: "hsl(42 60% 92%)",
		c3: "hsl(38 55% 82%)",
		c4: "hsl(34 45% 72%)",
		accent: "hsl(38 70% 80%)",
	},
	// Blue
	{
		c1: "hsl(212 72% 56%)",
		c2: "hsl(228 78% 60%)",
		c3: "hsl(202 55% 30%)",
		c4: "hsl(220 65% 40%)",
		accent: "hsl(215 78% 54%)",
	},
	// Green
	{
		c1: "hsl(142 60% 48%)",
		c2: "hsl(158 65% 42%)",
		c3: "hsl(128 50% 28%)",
		c4: "hsl(148 55% 34%)",
		accent: "hsl(140 70% 46%)",
	},
	// Purple
	{
		c1: "hsl(272 65% 58%)",
		c2: "hsl(288 70% 52%)",
		c3: "hsl(262 55% 30%)",
		c4: "hsl(280 60% 38%)",
		accent: "hsl(275 72% 56%)",
	},
	// Pink
	{
		c1: "hsl(332 75% 62%)",
		c2: "hsl(348 78% 58%)",
		c3: "hsl(318 60% 36%)",
		c4: "hsl(340 65% 44%)",
		accent: "hsl(335 78% 60%)",
	},
	// Teal
	{
		c1: "hsl(178 65% 46%)",
		c2: "hsl(192 70% 50%)",
		c3: "hsl(170 55% 26%)",
		c4: "hsl(186 60% 34%)",
		accent: "hsl(182 72% 48%)",
	},
	// Yellow
	{
		c1: "hsl(48 85% 58%)",
		c2: "hsl(38 80% 52%)",
		c3: "hsl(42 60% 32%)",
		c4: "hsl(44 70% 42%)",
		accent: "hsl(46 88% 56%)",
	},
];

export function paletteForLetter(name: string): OrgPalette {
	const ch = name.trim()[0]?.toLowerCase() ?? "";

	let index: number;
	if (ch >= "a" && ch <= "z") {
		index = (ch.charCodeAt(0) - 97) % PALETTES.length;
	} else if (ch >= "0" && ch <= "9") {
		index = Number.parseInt(ch, 10) % PALETTES.length;
	} else {
		index = 0;
	}

	return PALETTES[index];
}

export function orgConicGradient(palette: OrgPalette, angle = "0deg"): string {
	return `conic-gradient(from ${angle} at 50% 50%, ${palette.c1}, ${palette.c2}, ${palette.c3}, ${palette.c4}, ${palette.c2}, ${palette.c1})`;
}
