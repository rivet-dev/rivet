# Icons CLAUDE.md

## Overview

Icons come from Font Awesome Pro packages and a custom Font Awesome kit (`@awesome.me/kit-63db24046b`).

## Adding new custom kit icons

When a new icon has been uploaded to the custom Font Awesome kit:

1. Bump the kit version in `scripts/shared-utils.js` (`FA_PACKAGES_CONFIG["@awesome.me/kit-63db24046b"]`)
2. Run `FONTAWESOME_PACKAGE_TOKEN=<token> pnpm generate`
3. Commit the regenerated files (`manifest.json`, `src/index.gen.js`, `src/index.gen.ts`, `dist/index.js`, `dist/index.flat.js`, `dist/index.all.js`)

The generate script will warn you if the kit version is outdated. Always check for this warning.

## Dist file formats

- `dist/index.js` — monolithic bundle (all icons inlined, CJS-wrapper ESM format)
- `dist/index.flat.js` — flat ESM exports (`export const faX = {...}`) for Rollup tree-shaking
- `dist/index.all.js` — single default export object for runtime dynamic icon lookup
- `dist/index.gen.js` — esbuild output with free FA icons as external subpath imports

The package exports `dist/index.flat.js` (for `import { faTrash } from "@rivet-gg/icons"`) and `dist/index.all.js` (for `import("@rivet-gg/icons/all")`). The flat format enables Rollup to tree-shake unused icons. The all-icons registry is used only when all icons need to be available at runtime for dynamic lookup (e.g., actor icon picker).
