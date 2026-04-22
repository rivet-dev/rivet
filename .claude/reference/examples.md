# Examples reference

Rules for examples under `/examples/` and Vercel mirrors.

## Templates

- All example READMEs in `/examples/` follow the format defined in `.claude/resources/EXAMPLE_TEMPLATE.md`.

## Vercel mirror

- When adding or updating examples, ensure the Vercel equivalent is also modified (if applicable) to keep parity between local and Vercel examples.
- Regenerate with `./scripts/vercel-examples/generate-vercel-examples.ts` after making changes to examples.
- To skip Vercel generation for a specific example, add `"skipVercel": true` to the `template` object in the example's `package.json`.

## Common Vercel regen errors

- `error TS2688: Cannot find type definition file for 'vite/client'.` and `node_modules missing` warnings are fixed by running `pnpm install` before type checks. Regenerated examples need dependencies reinstalled.
