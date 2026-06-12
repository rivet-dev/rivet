#!/usr/bin/env node
/**
 * Builds `dist/inspector-tab/styles.css` by concatenating three inputs:
 *
 *   1. `packages/components/public/theme.css` — the dashboard's token
 *      source of truth. Carries both `:root` (light) and
 *      `:root[class~="dark"]` (dark) blocks so the tab tracks whichever
 *      theme the dashboard signals via `v1Init.theme`.
 *   2. `scripts/inspector-tab-aliases.css` — hand-maintained alias layer
 *      that re-exports the dashboard tokens under a `--rivet-` prefix
 *      (so authors can use them without colliding with their own
 *      unprefixed CSS variables), plus a small reset + utility classes.
 *   3. A short header comment + a Google Fonts `@import` (placed at the
 *      top because `@import` must precede other rules).
 *
 * No CSS parsing — just string concatenation.
 */

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = resolve(__dirname, "..");

const THEME_CSS_PATH = join(
	FRONTEND_ROOT,
	"packages/components/public/theme.css",
);
const ALIASES_CSS_PATH = join(__dirname, "inspector-tab-aliases.css");
const OUTPUT_DIR = join(FRONTEND_ROOT, "dist/inspector-tab");
const OUTPUT_PATH = join(OUTPUT_DIR, "styles.css");

const FONT_IMPORT = `@import url('https://fonts.googleapis.com/css2?family=IBM+Plex+Sans:wght@400;500;600;700&family=IBM+Plex+Mono:wght@400;500;600&display=swap');`;

const HEADER = `/*!
 * Rivet inspector custom-tab stylesheet.
 *
 * Auto-generated from \`packages/components/public/theme.css\` (dashboard
 * tokens) and \`scripts/inspector-tab-aliases.css\` (rivet-prefix aliases +
 * reset). Do not edit by hand — rerun
 * \`scripts/generate-inspector-tab-css.mjs\` from frontend/.
 *
 * Use it from a custom tab bundle. The href MUST be relative because
 * \`/inspector/tab.css\` is mounted per-actor under
 * \`/gateway/<actorId>/inspector/tab.css\` — an absolute path would
 * resolve to the engine root and 404. Two directories up from
 * \`/inspector/custom-tabs/<id>/\` lands on the right route:
 *
 *   <link rel="stylesheet" href="../../tab.css">
 *
 * Theme switching: tokens follow the dashboard's mechanism — \`:root\`
 * carries the light defaults and \`:root[class~="dark"]\` overrides with
 * dark values. The dashboard passes the active theme via the \`v1Init\`
 * postMessage (\`theme: "light" | "dark"\`); apply it by toggling the
 * \`dark\` class on \`<html>\` inside the tab.
 */`;

function main() {
	for (const p of [THEME_CSS_PATH, ALIASES_CSS_PATH]) {
		if (!existsSync(p)) {
			console.error(`Missing source file: ${p}`);
			process.exit(1);
		}
	}

	// theme.css starts with its own font @import. We strip it because we
	// emit our own font @import in the header (@import must come first in
	// the output stylesheet, before any other rules). A naive regex on
	// "@import...;" would terminate at the first semicolon inside the URL
	// (the Google Fonts URL contains `;`-separated weight params), so we
	// drop the entire first line instead.
	const themeCss = readFileSync(THEME_CSS_PATH, "utf8")
		.replace(/^@import\s+url\([^)]*\)\s*;\s*\n?/, "");
	const aliasesCss = readFileSync(ALIASES_CSS_PATH, "utf8");

	const stylesheet = [
		HEADER,
		FONT_IMPORT,
		"",
		"/* === Dashboard tokens (from packages/components/public/theme.css) === */",
		themeCss.trim(),
		"",
		"/* === Inspector-tab alias layer + reset (from scripts/inspector-tab-aliases.css) === */",
		aliasesCss.trim(),
		"",
	].join("\n");

	mkdirSync(OUTPUT_DIR, { recursive: true });
	writeFileSync(OUTPUT_PATH, stylesheet);

	console.log(`Wrote ${OUTPUT_PATH} (${stylesheet.length} bytes)`);
}

main();
