import { defineConfig } from 'astro/config';
import mdx from '@astrojs/mdx';
import react from '@astrojs/react';
import tailwind from '@astrojs/tailwind';
import sitemap from '@astrojs/sitemap';

import { remarkPlugins } from './src/mdx/remark';
import { rehypePlugins } from './src/mdx/rehype';
import { generateRoutes } from './src/integrations/generate-routes';
import { typecheckCodeBlocks } from './src/integrations/typecheck-code-blocks';
import { skillVersion } from './src/integrations/skill-version';

export default defineConfig({
	site: 'https://rivet.dev',
	output: 'static',
	trailingSlash: 'ignore',
	// SEO Redirects - Astro generates HTML redirect files for static builds
	// These work in dev server and all deployment platforms (Vercel, Netlify, Cloudflare, etc.)
	redirects: {
		'/docs': '/docs/actors/',
		// Documentation restructure
		'/docs/setup': '/docs/actors/quickstart/',
		'/docs/actors/queue': '/docs/actors/queues/',
		'/docs/actors/websockets': '/docs/actors/websocket-handler/',
		'/docs/actors/http': '/docs/actors/http-api/',
		'/docs/actors/run': '/docs/actors/lifecycle/',
		'/docs/actors/scheduling': '/docs/actors/schedule/',
		'/docs/actors/external-sql': '/docs/actors/postgres/',
		'/docs/actors/raw-sql': '/docs/actors/persistence/',
		// Platform docs moved to clients/connect
		'/docs/platforms/react': '/docs/clients/react/',
		'/docs/platforms/next-js': '/docs/clients/javascript/',
		'/docs/platforms/cloudflare-workers': '/docs/connect/cloudflare-workers/',
		// Registry configuration moved
		'/docs/connect/registry-configuration': '/docs/general/registry-configuration/',
		// Cloud docs removed - redirect to relevant sections
		'/docs/cloud': '/docs/self-hosting/',
		'/docs/cloud/api/actors/create': '/docs/actors/',
		'/docs/cloud/api/routes/update': '/docs/actors/',
		'/docs/cloud/self-hosting/single-container': '/docs/self-hosting/docker-container/',
		// Next.js client redirect (linked from homepage)
		'/docs/clients/next-js': '/docs/clients/javascript/',
		// Self-hosting redirect
		'/docs/general/self-hosting': '/docs/self-hosting/',
	},
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
