/**
 * Rivet docs configuration for @rivet-dev/docs-theme.
 *
 * Rivet is the project the theme was forked from, so it consumes the theme's
 * MDX pipeline (remark/rehype/Shiki) + route generation via `docsTheme()` while
 * keeping its own heavily-branded Header/Footer/BaseLayout and its in-repo
 * sitemap data (`src/sitemap/mod.ts`). Those rivet-owned components read rivet's
 * own data sources, not this virtual config, so only the identity fields the
 * theme's shared components might read are needed here.
 *
 * @type {import('@rivet-dev/docs-theme').SiteConfig}
 */
export const siteConfig = {
	product: "Rivet",
	siteUrl: "https://rivet.dev",
	repo: "rivet-dev/rivet",
	editPath: "website/",
};
