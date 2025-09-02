import twContainerQueries from "@tailwindcss/container-queries";
import twTypography from "@tailwindcss/typography";
import plugin from "tailwindcss/plugin";
import type { Config } from "tailwindcss/types/config";
import twAnmiate from "tailwindcss-animate";
import * as styleHelpers from "./ui/helpers/index";

const safelistMap: Array<[string, readonly string[], { useDash?: boolean }]> = [
	["m(x|y|l|r|t|b)", styleHelpers.MARGIN_VALUES, {}],
	["m", styleHelpers.MARGIN_VALUES, {}],
	["p", styleHelpers.PADDING_VALUES, {}],
	["p(x|y|l|r|t|b)", styleHelpers.PADDING_VALUES, {}],
	["gap", styleHelpers.GAP_VALUES, {}],
	["flex", styleHelpers.FLEX_DIRECTION_VALUES, {}],
	["justify", styleHelpers.JUSTIFY_CONTENT_VALUES, {}],
	["items", styleHelpers.ALIGN_ITEMS_VALUES, {}],
	["grid-cols", styleHelpers.GRID_COLUMNS_VALUES, {}],
	["w", styleHelpers.WIDTH_VALUES, {}],
	["flex", styleHelpers.FLEX_VALUES, {}],
	["min-h", styleHelpers.MIN_HEIGHT_VALUES, {}],
	["min-w", styleHelpers.MIN_WIDTH_VALUES, {}],
	["text", styleHelpers.TEXT_ALIGN_VALUES, {}],
	["h", styleHelpers.HEIGHT_VALUES, {}],
	["", styleHelpers.DISPLAY_VALUES, { useDash: false }],
];

const config = {
	content: [],
	darkMode: ["class"],
	prefix: "",
	safelist: safelistMap.map(([pattern, values, { useDash = true }]) => {
		const separator = useDash ? "-" : "";
		return {
			pattern: new RegExp(`${pattern}${separator}(${values.join("|")})`),
			variants: ["xl", "lg", "md", "sm"],
		};
	}),
	theme: {
		container: {
			center: true,
			padding: "2rem",
			screens: {
				"2xl": "1400px",
			},
		},
		extend: {
			fontFamily: {
				"mono-console": [
					"Consolas",
					"Lucida Console",
					"Courier New",
					"monospace",
				],
			},
			data: {
				active: 'status~="active"',
				open: 'state*="open"',
			},
			aria: {
				"current-page": "current='page'",
			},
			colors: {
				border: "hsl(var(--border))",
				input: "hsl(var(--input))",
				ring: "hsl(var(--ring))",
				background: {
					DEFAULT: "hsl(var(--background))",
					main: "hsl(var(--background-main))",
				},
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
				warning: {
					DEFAULT: "hsl(var(--warning))",
					foreground: "hsl(var(--warning-foreground))",
				},
				muted: {
					DEFAULT: "hsl(var(--muted))",
					foreground: "hsl(var(--muted-foreground))",
					destructive: "hsl(var(--muted-destructive))",
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
				"bounce-x": {
					"0%,100%": { transform: "translateX(25%)" },
					"50%": { transform: "translateX(-25%)" },
				},
				shake: {
					"10%, 90%": {
						transform: "translate3d(-1px, 0, 0)",
					},
					"20%, 80%": {
						transform: "translate3d(2px, 0, 0)",
					},
					"30%, 50%, 70%": {
						transform: "translate3d(-4px, 0, 0)",
					},
					"40%, 60%": {
						transform: "translate3d(4px, 0, 0)",
					},
				},
			},
			animation: {
				shake: "shake 0.82s cubic-bezier(.36,.07,.19,.97) both",
				"accordion-down": "accordion-down 0.2s ease-out",
				"accordion-up": "accordion-up 0.2s ease-out",
				"caret-blink": "caret-blink 1.25s ease-out infinite",
				"bounce-x": "bounce-x 5s ease infinite",
			},

			typography: {
				DEFAULT: {
					css: {
						"--tw-prose-invert-body": "hsl(var(--foreground))",
						"--tw-prose-invert-headings": "hsl(var(--foreground))",
						"--tw-prose-invert-lead": "hsl(var(--foreground))",
						"--tw-prose-invert-links": "hsl(var(--foreground))",
						"--tw-prose-invert-bold": "hsl(var(--foreground))",
						"--tw-prose-invert-counters": "hsl(var(--foreground))",
						"--tw-prose-invert-bullets": "hsl(var(--foreground))",
						"--tw-prose-invert-hr": "hsl(var(--border))",
						"--tw-prose-invert-quotes": "hsl(var(--foreground))",
						"--tw-prose-invert-quote-borders": "hsl(var(--border))",
						"--tw-prose-invert-captions": "hsl(var(--foreground))",
						"--tw-prose-invert-code": "hsl(var(--foreground))",
						"--tw-prose-invert-pre-code": "hsl(var(--foreground))",
						"--tw-prose-invert-pre-bg": "rgb(0 0 0 / 50%)",
						"--tw-prose-invert-th-borders": "hsl(var(--border))",
						"--tw-prose-invert-td-borders": "hsl(var(--border))",
					},
				},
			},
		},
	},
	plugins: [
		twAnmiate,
		twContainerQueries,
		twTypography(),
		plugin(({ addUtilities }) => {
			addUtilities({
				".field-sizing-content": {
					"field-sizing": "content",
				},
			});
		}),
	],
} satisfies Config;

export default config;
