import { defineConfig } from 'astro/config';
import mdx from '@astrojs/mdx';
import react from '@astrojs/react';
import tailwind from '@astrojs/tailwind';
import sitemap from '@astrojs/sitemap';
import sentry from "@sentry/astro";

import { remarkPlugins } from './src/mdx/remark';
import { rehypePlugins } from './src/mdx/rehype';
import { generateRoutes } from './src/integrations/generate-routes';
import { typecheckCodeBlocks } from './src/integrations/typecheck-code-blocks';
import { skillVersion } from './src/integrations/skill-version';
import { redirects } from './redirects.mjs';


export default defineConfig({
	site: 'https://rivet.dev',
	output: 'static',
	trailingSlash: 'ignore',
	image: {
		// Allow build-time optimization of artwork hosted on the assets CDN.
		domains: ['assets.rivet.dev'],
	},
	// SEO Redirects - Astro generates HTML redirect files for static builds and
	// serves them on the dev server. The same map drives real HTTP 301s at the
	// Caddy layer in production (see scripts/generate-caddy-redirects.mjs), so it
	// lives in a shared module to keep the two from drifting.
	redirects,
	prefetch: {
		prefetchAll: true,
		defaultStrategy: 'hover',
	},
	build: {
		assets: '_astro',
		format: 'directory',
	},
	markdown: {
		syntaxHighlight: false,
		remarkPlugins,
		rehypePlugins,
	},
	integrations: [
		skillVersion(),
		typecheckCodeBlocks(),
		generateRoutes(),
		mdx({
			syntaxHighlight: false,
			remarkPlugins,
			rehypePlugins,
		}),
		react(),
		tailwind({
			applyBaseStyles: false,
		}),
		sitemap({
			// Cookbooks and comparison guides are intentionally hidden from the site
			// and kept out of SEO, so exclude them from the sitemap.
			filter: (page) =>
				!page.includes('/api/') &&
				!page.includes('/internal/') &&
				!page.includes('/cookbook') &&
				!page.includes('/compare'),
		}),
		sentry({
      		project: "website",
      		org: "rivet-gaming",
			authToken: process.env.SENTRY_AUTH_TOKEN,
		}),
	],
	vite: {
		// Mermaid is large and only dynamically imported by MermaidScript, so Vite
		// discovers it late and re-optimizes mid-session, which invalidates the
		// in-flight chunk and serves a 504 "Outdated Optimize Dep" for every
		// diagram. Pre-bundling it at server start keeps its chunk hash stable.
		optimizeDeps: {
			include: ['mermaid'],
		},
		ssr: {
			noExternal: ['@rivet-gg/components', '@rivet-gg/icons'],
		},
		server: {
			fs: {
				// Allow serving files from the monorepo root for artifacts
				allow: ['..'],
			},
		},
	},
});
