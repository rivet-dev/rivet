/** @type {import('tailwindcss').Config} */
module.exports = {
	darkMode: ["class"],
	// Restricting the content to the components folder
	// forces developers to use ui components for styling instead of utility classes
	content: ["../../node_modules/@rivet-gg/components/**/*.{ts,tsx}"],
	prefix: "",
	safelist: [
		{
			pattern: /m(x|y|l|r|t|b)-(0|2|4|6|8|10)/,
			variants: ["xl", "lg", "md", "sm"],
		},
		{
			pattern: /m-(0|2|4|6|8|10)/,
			variants: ["xl", "lg", "md", "sm"],
		},
		{
			pattern: /p-(0|2|4|6|8|10)/,
			variants: ["xl", "lg", "md", "sm"],
		},
		{
			pattern: /p(x|y|l|r|t|b)-(0|2|4|6|8|10)/,
			variants: ["xl", "lg", "md", "sm"],
		},
		{
			pattern: /gap-(0|1|2|4|6|8|10)/,
			variants: ["xl", "lg", "md", "sm"],
		},
		{
			pattern: /flex-(row|col|col-reverse|row-reverse)/,
			variants: ["xl", "lg", "md", "sm"],
		},
		{
			pattern: /justify-(start|end|center|between|around)/,
			variants: ["xl", "lg", "md", "sm"],
		},
		{
			pattern: /items-(start|end|center|baseline|stretch)/,
			variants: ["xl", "lg", "md", "sm"],
		},
		{
			pattern: /grid-cols-(1|2|3|4|5|6)/,
			variants: ["xl", "lg", "md", "sm"],
		},
		{
			pattern: /w-(1\/3|2\/3|full)/,
			variants: ["xl", "lg", "md", "sm"],
		},
	],
	theme: {
		container: {
			center: true,
			padding: "2rem",
			screens: {
				"2xl": "1400px",
			},
		},
		extend: {
			colors: {
				border: "hsl(var(--border))",
				input: "hsl(var(--input))",
				ring: "hsl(var(--ring))",
				background: "hsl(var(--background))",
				foreground: "hsl(var(--foreground))",
				primary: {
					DEFAULT: "hsl(var(--primary))",
					foreground: "hsl(var(--primary-foreground))",
				},
				secondary: {
					DEFAULT: "hsl(var(--secondary))",
					foreground: "hsl(var(--secondary-foreground))",
				},
				destructive: {
					DEFAULT: "hsl(var(--destructive))",
					foreground: "hsl(var(--destructive-foreground))",
				},
				muted: {
					DEFAULT: "hsl(var(--muted))",
					foreground: "hsl(var(--muted-foreground))",
				},
				accent: {
					DEFAULT: "hsl(var(--accent))",
					foreground: "hsl(var(--accent-foreground))",
				},
				popover: {
					DEFAULT: "hsl(var(--popover))",
					foreground: "hsl(var(--popover-foreground))",
				},
				card: {
					DEFAULT: "hsl(var(--card))",
					foreground: "hsl(var(--card-foreground))",
				},
			},
			borderRadius: {
				lg: "var(--radius)",
				md: "calc(var(--radius) - 2px)",
				sm: "calc(var(--radius) - 4px)",
			},
			keyframes: {
				"accordion-down": {
					from: { height: "0" },
					to: { height: "var(--radix-accordion-content-height)" },
				},
				"accordion-up": {
					from: { height: "var(--radix-accordion-content-height)" },
					to: { height: "0" },
				},
				"caret-blink": {
					"0%,70%,100%": { opacity: "1" },
					"20%,50%": { opacity: "0" },
				},
			},
			animation: {
				"accordion-down": "accordion-down 0.2s ease-out",
				"accordion-up": "accordion-up 0.2s ease-out",
				"caret-blink": "caret-blink 1.25s ease-out infinite",
			},
		},
	},
	plugins: [require("tailwindcss-animate")],
};
