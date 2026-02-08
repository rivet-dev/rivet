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
		// typecheckCodeBlocks(), // Temporarily disabled due to rivetkit build issues
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
