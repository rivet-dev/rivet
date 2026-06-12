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
			filter: (page) => !page.includes('/api/') && !page.includes('/internal/'),
		}),
		sentry({
      		project: "website",
      		org: "rivet-gaming",
			authToken: process.env.SENTRY_AUTH_TOKEN,
		}),
	],
	vite: {
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
