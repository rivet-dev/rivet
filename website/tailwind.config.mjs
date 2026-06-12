import defaultTheme from "tailwindcss/defaultTheme";

/** @type {import('tailwindcss').Config} */
export default {
	content: [
		"./src/**/*.{astro,ts,tsx,jsx,js,mdx,md}",
		"./node_modules/@rivet-gg/components/src/**/*.{ts,tsx}",
	],
	safelist: [
		'learn-container',
		'texture-overlay',
		'font-display',
		'font-serif',
		'font-mono',
		'narrative-drop-cap',
		// Sonner toast classes
		'bg-card',
		'text-foreground',
		'border-border',
		'shadow-lg',
		'border',
		'rounded-md',
		'text-muted-foreground',
		'bg-primary',
		'text-primary-foreground',
		'bg-muted',
	],
	theme: {
		extend: {
			colors: {
				'background': '#000000',
				'text-primary': '#FAFAFA',
				'text-secondary': '#A0A0A0',
				'border': '#252525',
				'accent': '#FF4500',
				// Porcelain editorial palette. Marketing pages are light: a cool
				// porcelain field with warm ink, warm mats, and forest green
				// structure. Orange remains the single CTA spark per page.
				'paper': '#EFEFEF',
				'paper-deep': '#DCDCDE',
				'paper-mid': '#E3E3E5',
				'mat': '#EFE9DC',
				'ink': '#1B1916',
				'ink-soft': '#56524A',
				'ink-faint': '#8A8478',
				'cream': '#F4F1E7',
				'pine': '#2E4034',
				'olive': '#5C6B4F',
				'sage': '#93A286',
				'accent-deep': '#D63E00',
				'code-keyword': '#c084fc',
				'code-function': '#60a5fa',
				'code-string': '#4ade80',
				'code-comment': '#737373',
			},
			fontFamily: {
				sans: ["Manrope", ...defaultTheme.fontFamily.sans],
				'heading': ['Manrope', 'sans-serif'],
				'mono': ['"JetBrains Mono"', 'monospace'],
			},
			animation: {
				'fade-in-up': 'fade-in-up 0.8s ease-out forwards',
				'hero-line': 'hero-line 1s cubic-bezier(0.19, 1, 0.22, 1) forwards',
				'hero-p': 'hero-p 0.8s ease-out 0.6s forwards',
				'hero-cta': 'hero-p 0.8s ease-out 0.8s forwards',
				'hero-visual': 'hero-p 0.8s ease-out 1s forwards',
				'infinite-scroll': 'infinite-scroll 25s linear infinite',
				'pulse-slow': 'pulse-slow 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
				'spin-slow': 'spin 120s linear infinite',
			},
			keyframes: {
				'fade-in-up': {
					'from': { opacity: '0', transform: 'translateY(24px)' },
					'to': { opacity: '1', transform: 'translateY(0)' },
				},
				'hero-line': {
					'0%': { opacity: '0', transform: 'translateY(100%) skewY(6deg)' },
					'100%': { opacity: '1', transform: 'translateY(0) skewY(0deg)' },
				},
				'hero-p': {
					'from': { opacity: '0', transform: 'translateY(20px)' },
					'to': { opacity: '1', transform: 'translateY(0)' },
				},
				'infinite-scroll': {
					'from': { transform: 'translateX(0)' },
					'to': { transform: 'translateX(-50%)' },
				},
				'pulse-slow': {
					'50%': { opacity: '.5' },
				},
			},
			gridTemplateColumns: {
				docs: "20rem 1fr",
				"docs-no-sidebar": "1fr",
			},
			typography: ({ theme }) => ({
				DEFAULT: {
					css: {
						// Light (non-inverted) prose palette for porcelain
						// surfaces. Dark shells use prose-invert, which reads
						// the --tw-prose-invert-* values below instead.
						"--tw-prose-body": "#56524A",
						"--tw-prose-headings": "#1B1916",
						"--tw-prose-lead": "#56524A",
						"--tw-prose-links": "#2E4034",
						"--tw-prose-bold": "#1B1916",
						"--tw-prose-counters": "#8A8478",
						"--tw-prose-bullets": "#8A8478",
						"--tw-prose-hr": "rgba(27, 25, 22, 0.1)",
						"--tw-prose-quotes": "#56524A",
						"--tw-prose-quote-borders": "#2E4034",
						"--tw-prose-captions": "#8A8478",
						"--tw-prose-code": "#1B1916",
						"--tw-prose-pre-code": "#F4F1E7",
						"--tw-prose-pre-bg": "#1B1916",
						"--tw-prose-th-borders": "rgba(27, 25, 22, 0.15)",
						"--tw-prose-td-borders": "rgba(27, 25, 22, 0.1)",
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
			top: {
				header: "var(--header-height, 3.5rem)",
			},
			scrollMargin: {
				header: "calc(var(--header-height, 3.5rem) + 1rem)",
			},
			maxHeight: {
				content: "calc(100vh - var(--header-height, 3.5rem))",
			},
			maxWidth: {
				'prose-docs': '52rem',
			},
		},
	},
	presets: [require("@rivet-gg/components/tailwind-base").default],
	plugins: [require("@tailwindcss/typography")],
};
