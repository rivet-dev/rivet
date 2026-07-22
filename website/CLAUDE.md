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

- Always store docs code examples under `examples/docs/` and render them with `<CodeSnippet>`. Do not add inline fenced code blocks to docs MDX files.
- Type-check all TypeScript code blocks in `website/src/content/docs/**/*.mdx` before release, because any failing snippet fails the website build.
- Document `onStateChange` as read-only against `c.state`; use `vars` for callback counters or derived runtime-only values.
- Connect page cards and sidebar entries come from `frontend/packages/shared-data/src/deploy.ts`.

### Required for every TypeScript snippet

- Include all required imports.
- Define all referenced variables and types.
- Avoid placeholders or incomplete code that cannot compile.
- Use `@nocheck` only when a snippet intentionally documents API not available on this branch yet.

### Multi-file examples

- Use `<CodeGroup workspace>` for any example that spans multiple files (for example `registry.ts` + `client.ts`).
- Follow these rules for workspace code groups.

- Store every file under `examples/docs/` and render each one with `<CodeSnippet file="..." title="..." />` inside the group.
- Treat files as real modules in the same directory and use relative imports (for example `import type { registry } from "./registry"`).
- Do not split related multi-file examples into separate non-workspace snippets.

- If any code block fails type checking, the build fails.

## Typography

- Marketing headings and card titles use `font-medium` (h1 heroes, section h2s, h3/h4 card titles, FAQ headings, price figures). Do not introduce `font-normal` headings.
- `font-normal` remains correct only for deliberate de-emphasis: table `th` de-bolding, quiet `dt` labels, and input-like UI (TypesenseSearch).
- Hero h1s and section h2s use the class constants in `src/components/marketing/typography.tsx` (`tracking-[-0.015em]`, h1 `leading-[1.06]`). Do not hand-write `tracking-tight` on headings at `text-2xl` or larger; `tracking-tight` stays acceptable on `text-base` and smaller card titles.
- Manrope is the only marketing typeface; JetBrains Mono covers code, eyebrows, and captions. No serif headings on marketing pages; the Learn section's serif treatment is the only exception.
- Section labels use the eyebrow convention (`EYEBROW_CLASS` or `editorial/Eyebrow`): mono, uppercase, `tracking-[0.18em]`, pine on porcelain, sage inside ink panels. Plate and panel captions use `CAPTION_CLASS` (`Fig. NN — ...`).

## Theme

- Marketing pages and docs are light: cool porcelain (`paper`, `#EFEFEF`) with a `paper-deep` radial pooling bottom-left (`.depth-wash`) and a fine grain (`.paper-grain`). Warmth comes from warm-black `ink` text, classical imagery, and oil-paint textures, never from synthetic color gradients. Do not use the cream `mat` token as a surface/background design element on light surfaces (panels, dropdowns, badges, plate frames); use `paper`/`paper-mid`/`white` or `ink`-tint neutrals instead. `cream` stays valid only as the off-white text/fill inside dark `ink` panels. Docs paint the same porcelain field, render prose via `Prose surface="paper"`, and use `pine` for the active sidebar/TOC selected state; only the Learn section keeps a dark shell, and no other page may introduce a dark base.
- Dark `ink` panels (`editorial/InkPanel`) are reserved for code, terminal, screenshot, and data moments. Code and data plates stay flat ink; the oil-texture backdrop (`textureSrc`) is for editorial moments only (CTA colophon, 404).
- Orange is the spark: at most one `accent`/ember CTA per page. White text sits only on `accent-deep` (`#D63E00`) or `ink` fills, never on `accent`. Pine (`#2E4034`) is the structural color for links, eyebrows, diagrams, and selected states; sage (`#93A286`) replaces it inside ink panels.
- No drop shadows on marketing cards or imagery; depth comes from `border-ink/10..25` hairlines, `bg-white/55` card fills, and `paper-mid`/`paper-deep` layering (inside ink panels: `border-cream/10..15`). Shadows stay acceptable on functional overlays (dropdowns, tooltips, modals) and the header's glass inset highlight.
- `ink-faint` text is for captions and metadata at 12px+; body copy uses `ink-soft` or `ink`.
