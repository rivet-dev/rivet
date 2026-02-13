# Website CLAUDE.md

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
