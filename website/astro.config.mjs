import { defineConfig } from 'astro/config';
import tailwind from '@astrojs/tailwind';
import sitemap from '@astrojs/sitemap';
import sentry from "@sentry/astro";

// Rivet consumes the shared docs framework it was forked from. docsTheme()
// provides react + mdx + the remark/rehype/Shiki pipeline + route generation +
// the virtual config — replacing rivet's previously-inlined copies of those.
import { docsTheme } from '@rivet-dev/docs-theme';
import { siteConfig } from './docs.config.mjs';
import { skillVersion } from './src/integrations/skill-version';
import { redirects } from './redirects.mjs';

// Wildcard sub-path redirects (`wildcardRedirects` in redirects.mjs) are applied
// only at the Caddy layer in production. Astro's static output treats a redirect
// key containing a rest param (`[...slug]`) as a dynamic route that needs
// `getStaticPaths`, so feeding external-URL wildcards here aborts the build. The
// explicit non-wildcard entries below still cover every real former route on the
// dev server; deep sub-paths fall through to Caddy in production.
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
	redirects: redirects,
	prefetch: {
		prefetchAll: true,
		defaultStrategy: 'hover',
	},
	build: {
		assets: '_astro',
		format: 'directory',
	},
	// markdown (.md) remark/rehype + syntaxHighlight:false are configured by
	// docsTheme()'s config integration; .mdx is handled by the theme's mdx().
	integrations: [
		skillVersion(),
		...docsTheme(siteConfig),
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
