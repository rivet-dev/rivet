# Icons CLAUDE.md

## Overview

Icons come from Font Awesome Pro packages and a custom Font Awesome kit (`@awesome.me/kit-63db24046b`).

## Adding new custom kit icons

When a new icon has been uploaded to the custom Font Awesome kit:

1. Bump the kit version in `scripts/shared-utils.js` (`FA_PACKAGES_CONFIG["@awesome.me/kit-63db24046b"]`)
2. Run `FONTAWESOME_PACKAGE_TOKEN=<token> pnpm generate`
3. Commit the regenerated files (`manifest.json`, `src/index.gen.js`, `src/index.gen.ts`, `dist/index.js`)

The generate script will warn you if the kit version is outdated. Always check for this warning.
