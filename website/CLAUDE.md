# Website CLAUDE.md

## Icons

To add or update icons, see `frontend/packages/icons/CLAUDE.md`.

## Registry Integration Icons

Integration entries in `website/src/data/registry.ts` display icons on the registry page and detail pages. Each entry uses either an `image` (SVG file path) or an `icon` (Font Awesome icon).

### When to use each

- **`image`** (SVG file in `website/public/images/registry/`): Use for products and companies that have their own logo (e.g. Docker, Vercel, E2B).
- **`icon`** (Font Awesome from `@rivet-gg/icons`): Use for generic/non-product items that don't have a brand logo (e.g. Filesystem, Browser, SQLite).

### Fetching product logos

When adding a new product integration:

1. Search for the product's official SVG logo. Try these sources in order:
   - `https://simpleicons.org/icons/{name}.svg` (then apply the brand color)
   - The product's website favicon or press kit
   - Their GitHub organization avatar
2. Save the SVG to `website/public/images/registry/{slug}.svg`.
3. **Use actual brand colors.** Do not convert logos to white/monochrome. Logos display on a dark background, so avoid dark/black logos. If a logo is black-only, find the dark-mode variant.
4. The carousel selector at the top of the registry page applies a monochrome filter automatically. The colored version displays in the main card and detail pages.

### Font Awesome icons

Import from `@rivet-gg/icons`. The full Font Awesome Pro library is available. Common choices for registry items:

- `faFloppyDisk` - filesystem/storage
- `faGlobe` - web/browser/network
- `faDatabase` - database
- `faSqlite` / `faPostgresql` - specific databases
- `faBrain` - AI/memory
- `faDesktop` - local/desktop
- `faCode` - code/interpreter

## Code Blocks

All TypeScript code blocks in documentation files (`website/src/content/docs/**/*.mdx`) are type-checked before release. If any snippet fails, the website build fails.

### Required for every TypeScript snippet

- Include all required imports.
- Define all referenced variables and types.
- Avoid placeholders or incomplete code that cannot compile.
- Use `@nocheck` only when a snippet intentionally documents API not available on this branch yet.

### Multi-file examples

Use `<CodeGroup workspace>` for any example that spans multiple files (for example `registry.ts` + `client.ts`).

Rules:

- Every file in a workspace group must include a simple inline code fence title, for example `ts registry.ts` after the opening triple backticks.
- Treat files as real modules in the same directory and use relative imports (for example `import type { registry } from "./registry"`).
- Mark setup-only files with `@hide` when you want them type-checked but not prominently shown.
- Do not split related multi-file examples into separate non-workspace code blocks.

If a code block fails type checking, the build will fail.
