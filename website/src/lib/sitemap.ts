// Rivet consumes the shared docs theme it was forked from. The sitemap type
// contract (Sitemap/SidebarItem/SidebarSection/AnyPage) and the
// active-tab/page helpers (findActiveTab/findPageForHref) are owned by
// @rivet-dev/docs-theme; rivet supplies only the data in src/sitemap/mod.ts.
// This re-export keeps the in-repo `@/lib/sitemap` import path working while the
// single source of truth lives in the theme (the file was byte-identical to the
// theme's copy at inversion time).
export * from "@rivet-dev/docs-theme/sitemap";
