import defaultTheme from "tailwindcss/defaultTheme";
import type { Config } from "tailwindcss/types/config";

// Used by the new design
const config = {
	content: [
		"./src/**/*.{ts,tsx,jsx,js,mdx,md}",
		"../node_modules/@rivet-gg/components/**/*.{ts,tsx}",
	],
	theme: {
		extend: {
			fontFamily: {
				sans: ["Open Sans", ...defaultTheme.fontFamily.sans],
			},
			gridTemplateColumns: {
				docs: "20rem 1fr",
				"docs-no-sidebar": "1fr",
			},
			typography: ({ theme }) => ({
				DEFAULT: {
					css: {
						"--tw-prose-invert-body":
							"hsl(var(--muted-foreground))",
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
						h1: {
							fontWeight: "600",
						},
						h2: {
							fontWeight: "600",
						},
						h3: {
							fontWeight: "600",
						},
						h4: {
							fontWeight: "600",
						},
						h5: {
							fontWeight: "600",
						},
						h6: {
							fontWeight: "600",
						},
						code: {
							fontSize: "inherit",
							fontWeight: "inherit",
						},
						"code::before": {
							content: "none",
						},
						"code::after": {
							content: "none",
						},
					},
				},
			}),
			spacing: {
				header: "var(--header-height, 3.5rem)",
			},
			scrollMargin: {
				header: "calc(var(--header-height, 3.5rem) + 1rem)",
			},
			maxHeight: {
				content: "calc(100vh - var(--header-height, 3.5rem))",
			},
		},
	},
	presets: [require("@rivet-gg/components/tailwind-base").default],
	plugins: [require("@tailwindcss/typography")],
} satisfies Config;

export default config;
