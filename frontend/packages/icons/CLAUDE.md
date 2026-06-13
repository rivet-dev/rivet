# Icons CLAUDE.md

## Overview

Icons come from Font Awesome Pro packages and a custom Font Awesome kit (`@awesome.me/kit-63db24046b`).

## Requesting a new icon

Custom kit icons (for example a company brand) cannot be uploaded by an agent. When you need one:

1. If you can find the relevant icon (for example a company logo), look up its SVG and download a single-color (monocolor, `fill="currentColor"`) version to a temp directory ready for the user to upload. Do not commit it to the repo.
2. Ask the user to upload it to the custom Font Awesome kit at https://fontawesome.com/kits/63db24046b/customicons.
3. Once the user confirms it is uploaded, repull the icons with the steps below, then consume the generated icon from `@rivet-gg/icons` instead of inlining an SVG path.

## Adding new custom kit icons

When a new icon has been uploaded to the custom Font Awesome kit:

1. Bump the kit version in `scripts/shared-utils.js` (`FA_PACKAGES_CONFIG["@awesome.me/kit-63db24046b"]`)
2. Run `FONTAWESOME_PACKAGE_TOKEN=<token> pnpm generate`
3. Commit the regenerated files (`manifest.json`, `src/index.gen.js`, `src/index.gen.ts`, `dist/index.js`)

The generate script will warn you if the kit version is outdated. Always check for this warning.
