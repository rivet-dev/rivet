# Website CLAUDE.md

## Code Blocks

All code blocks in documentation files (`website/src/content/docs/**/*.mdx`) are type-checked before release. Code blocks must be valid, compilable TypeScript.

This means:
- Include all necessary imports
- Define all variables and types used
- Avoid incomplete snippets with undefined references
- Use proper type annotations where needed

If a code block fails type checking, the build will fail.
