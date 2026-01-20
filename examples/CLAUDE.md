# examples/CLAUDE.md

Guidelines for creating and maintaining examples in this repository.

## README Format

All example READMEs must follow the template defined in `.claude/resources/EXAMPLE_TEMPLATE.md`. Key requirements:
- Use exact section headings: `## Getting Started`, `## Features`, `## Implementation`, `## Resources`, `## License`
- Include `## Prerequisites` only for non-obvious dependencies (API keys, external services)
- Focus features on RivetKit concepts demonstrated, not just app functionality
- Include GitHub source code links in Implementation section

## Project Structure

### Directory Layout

Examples with frontend (using vite-plugin-srvx):
```
example-name/
├── src/
│   ├── actors.ts       # Actor definitions and registry setup
│   └── server.ts       # Server entry point
├── frontend/
│   ├── App.tsx         # Main React component
│   └── main.tsx        # React entry point
├── tests/
│   └── *.test.ts       # Vitest tests
├── index.html          # HTML entry point (for Vite)
├── package.json
├── tsconfig.json
├── vite.config.ts
├── vitest.config.ts    # Only if tests exist
├── turbo.json
└── README.md
```

Examples with separate frontend/backend dev servers:
```
example-name/
├── src/
│   ├── actors.ts       # Actor definitions and registry setup
│   └── server.ts       # Server entry point
├── frontend/
│   ├── App.tsx
│   └── main.tsx
├── package.json
├── tsconfig.json
├── tsup.config.ts      # For backend bundling
├── vite.config.ts
├── vitest.config.ts    # Only if tests exist
├── turbo.json
└── README.md
```

Backend-only examples:
```
example-name/
├── src/
│   ├── actors.ts       # Actor definitions and registry setup
│   └── server.ts       # Server entry point
├── package.json
├── tsconfig.json
├── turbo.json
└── README.md
```

### Naming Conventions

- Actor definitions go in `src/actors.ts`
- Server entry point is always `src/server.ts`
- Frontend entry is `frontend/main.tsx` with main component in `frontend/App.tsx`
- Test files use `.test.ts` extension in `tests/` directory

## package.json

### Required Scripts

For examples with frontend (using vite-plugin-srvx):
```json
{
  "scripts": {
    "dev": "vite",
    "check-types": "tsc --noEmit",
    "test": "vitest run",
    "build": "vite build && vite build --mode server",
    "start": "srvx --static=public/ dist/server.js"
  }
}
```

For examples with separate frontend/backend dev servers:
```json
{
  "scripts": {
    "dev:backend": "srvx --import tsx src/server.ts",
    "dev:frontend": "vite",
    "dev": "concurrently \"npm run dev:backend\" \"npm run dev:frontend\"",
    "check-types": "tsc --noEmit",
    "test": "vitest run",
    "build:frontend": "vite build",
    "build:backend": "tsup",
    "build": "npm run build:backend && npm run build:frontend",
    "start": "srvx --static=../frontend/dist dist/server.js"
  }
}
```

For backend-only examples:
```json
{
  "scripts": {
    "dev": "npx srvx --import tsx src/server.ts",
    "start": "npx srvx --import tsx src/server.ts",
    "check-types": "tsc --noEmit",
    "build": "tsup"
  }
}
```

### Required Fields

```json
{
  "name": "example-name",
  "version": "2.0.21",
  "private": true,
  "type": "module",
  "stableVersion": "0.8.0",
  "template": {
    "technologies": ["react", "typescript"],
    "tags": ["real-time"],
    "frontendPort": 5173,
    "noFrontend": true  // Only for backend-only examples
  },
  "license": "MIT"
}
```

### Dependencies

- Use `"rivetkit": "*"` for the main RivetKit package
- Use `"@rivetkit/react": "*"` for React integration
- Common dev dependencies:
  - `tsx` for running TypeScript in development
  - `typescript` for type checking
  - `vite` and `@vitejs/plugin-react` for frontend
  - `vite-plugin-srvx` for unified dev server (when using vite-plugin-srvx pattern)
  - `vitest` for testing
  - `tsup` for bundling (only for separate frontend/backend examples)
  - `concurrently` for parallel dev servers (only for separate frontend/backend examples)
- Common production dependencies:
  - `hono` for the server framework (required for Vercel detection)
  - `srvx` for serving in production (used by `start` script)
  - `@hono/node-server` for Node.js HTTP server adapter
  - `@hono/node-ws` for Node.js WebSocket support

## Configuration Files

### tsconfig.json

```json
{
  "compilerOptions": {
    "target": "esnext",
    "lib": ["esnext", "dom"],
    "jsx": "react-jsx",
    "module": "esnext",
    "moduleResolution": "bundler",
    "types": ["node", "vite/client"],
    "noEmit": true,
    "strict": true,
    "skipLibCheck": true,
    "allowImportingTsExtensions": true,
    "rewriteRelativeImportExtensions": true
  },
  "include": ["src/**/*", "frontend/**/*", "tests/**/*"]
}
```

Notes:
- Include `"dom"` in lib for frontend examples
- Include `"vite/client"` in types when using Vite
- Omit `"frontend/**/*"` and `"tests/**/*"` from include if they don't exist
- `allowImportingTsExtensions` and `rewriteRelativeImportExtensions` enable ESM-compliant `.ts` imports

### tsup.config.ts

Only needed for examples with separate frontend/backend dev servers (not using vite-plugin-srvx):

```typescript
import { defineConfig } from "tsup";

export default defineConfig({
  entry: {
    server: "src/server.ts",
  },
  format: ["esm"],
  outDir: "dist",
  bundle: true,
  splitting: false,
  shims: true,
});
```

### vite.config.ts

For examples using vite-plugin-srvx (unified dev):
```typescript
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import srvx from "vite-plugin-srvx";

export default defineConfig({
  plugins: [react(), ...srvx({ entry: "src/server.ts" })],
});
```

For examples with separate dev servers:
```typescript
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  root: "frontend",
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  server: {
    host: "0.0.0.0",
    port: 5173,
    proxy: {
      "/api/rivet/": "http://localhost:3000",
    },
  },
});
```

### vitest.config.ts

```typescript
import { defineConfig } from "vitest/config";

export default defineConfig({
  server: {
    port: 5173,
  },
  test: {
    include: ["tests/**/*.test.ts"],
  },
});
```

### vercel.json

Vercel auto-detects Vite when it sees a `vite.config.ts` and ignores Hono. We must explicitly set the framework to Hono:

```json
{
  "framework": "hono"
}
```

### turbo.json

All examples should extend the root turbo config:
```json
{
  "$schema": "https://turbo.build/schema.json",
  "extends": ["//"]
}
```

### .gitignore

```
.actorcore
node_modules
```

## Source Code Patterns

### Actor Definitions (src/actors.ts)

```typescript
import { actor, setup } from "rivetkit";

// Export types for client usage
export type Message = { sender: string; text: string; timestamp: number };

export const chatRoom = actor({
  // Persistent state
  state: {
    messages: [] as Message[],
  },

  actions: {
    sendMessage: (c, sender: string, text: string) => {
      const message = { sender, text, timestamp: Date.now() };
      c.state.messages.push(message);
      c.broadcast("newMessage", message);
      return message;
    },
    getHistory: (c) => c.state.messages,
  },
});

// Registry setup - always export as `registry`
export const registry = setup({
  use: { chatRoom },
});
```

### Server Entry Point (src/server.ts)

You must explicitly import from `"hono"` for Vercel to detect the framework.

Minimum required:

```typescript
import { Hono } from "hono";
import { registry } from "./actors.ts";

const app = new Hono();
app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));
export default app;
```

With additional routes:

```typescript
import { Hono } from "hono";
import { registry } from "./actors.ts";

const app = new Hono();

app.get("/api/foo", (c) => c.text("bar"));

app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

export default app;
```

### React Frontend (frontend/App.tsx)

```typescript
import { createRivetKit } from "@rivetkit/react";
import type { registry } from "../src/actors.ts";

const { useActor } = createRivetKit<typeof registry>(`${location.origin}/api/rivet`);

export function App() {
  const actor = useActor({
    name: "actorName",
    key: ["key"],
  });

  // Use actor.connection for actions
  // Use actor.useEvent for event subscriptions
}
```

### React Entry Point (frontend/main.tsx)

```typescript
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App.tsx";

const root = document.getElementById("root");
if (!root) throw new Error("Root element not found");

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>
);
```

### Tests (tests/*.test.ts)

```typescript
import { setupTest } from "rivetkit/test";
import { expect, test } from "vitest";
import { registry } from "../src/actors.ts";

test("Description of test", async (ctx) => {
  const { client } = await setupTest(ctx, registry);
  const actor = client.actorName.getOrCreate(["key"]);

  // Test actor actions
  const result = await actor.someAction();
  expect(result).toEqual(expected);
});
```

## HTML Entry Point

For Vite-based examples:
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Example Title</title>
    <style>
        /* Inline styles for simplicity */
    </style>
</head>
<body>
    <div id="root"></div>
    <script type="module" src="/frontend/main.tsx"></script>
</body>
</html>
```

## ESM Import Requirements

All imports must be ESM-compliant with explicit `.ts` extensions for relative imports:

```typescript
// Correct
import { registry } from "./actors.ts";
import { someUtil } from "../utils/helper.ts";

// Incorrect
import { registry } from "./actors";
import { someUtil } from "../utils/helper";
```

This is enforced by the tsconfig options `allowImportingTsExtensions` and `rewriteRelativeImportExtensions`.

## Best Practices

1. **Keep examples minimal** - Focus on demonstrating specific RivetKit concepts
2. **Type safety** - Export types from actors for client usage, use `typeof registry` for type-safe clients
3. **Consistent naming** - Use `registry` for the setup export, match actor names to their purpose
4. **Real-time patterns** - Demonstrate `broadcast()` for events and `useEvent()` for subscriptions
5. **State management** - Show persistent state with clear before/after behavior
6. **Testing** - Use `setupTest()` from `rivetkit/test` for isolated actor testing
7. **Comments** - Include helpful comments linking to documentation (e.g., `// https://rivet.dev/docs/actors/state`)

## Vercel Examples

Vercel-optimized versions of examples are automatically generated using the script at `scripts/vercel-examples/generate-vercel-examples.ts`. These examples use the `hono/vercel` adapter and are configured specifically for Vercel serverless deployment.

### Generation Script

```bash
# Generate all changed examples (uses git diff)
npx tsx scripts/vercel-examples/generate-vercel-examples.ts

# Generate a specific example
npx tsx scripts/vercel-examples/generate-vercel-examples.ts --example hello-world

# Force regenerate all examples
npx tsx scripts/vercel-examples/generate-vercel-examples.ts --all

# Dry run (show what would be generated)
npx tsx scripts/vercel-examples/generate-vercel-examples.ts --dry-run
```

### Naming Convention

Vercel examples are placed at `examples/{original-name}-vercel/`. For example:
- `hello-world` → `hello-world-vercel`
- `chat-room` → `chat-room-vercel`

### Directory Layout

Vercel examples with frontend:
```
example-name-vercel/
├── api/
│   └── index.ts        # Hono handler with hono/vercel adapter
├── src/
│   ├── actors.ts       # Actor definitions (copied from origin)
│   └── server.ts       # Server entry point (copied from origin)
├── frontend/
│   ├── App.tsx         # React component (copied from origin)
│   └── main.tsx        # React entry point (copied from origin)
├── index.html          # HTML entry point (copied from origin)
├── package.json        # Modified for Vercel
├── tsconfig.json       # Modified for Vercel
├── vite.config.ts      # Simplified (no srvx)
├── vercel.json         # Vercel configuration
├── turbo.json
└── README.md           # With Vercel-specific note and deploy button
```

Vercel examples without frontend (API-only):
```
example-name-vercel/
├── api/
│   └── index.ts        # Hono handler with hono/vercel adapter
├── src/
│   ├── actors.ts
│   └── server.ts
├── package.json
├── tsconfig.json
├── vercel.json
├── turbo.json
└── README.md
```

### Key Files

#### api/index.ts

The API entry point uses the Hono Vercel adapter (built into the `hono` package):

```typescript
import { handle } from "hono/vercel";
import app from "../src/server.ts";

export default handle(app);
```

#### vercel.json

For examples with frontend:
```json
{
  "framework": "vite",
  "rewrites": [
    { "source": "/api/(.*)", "destination": "/api" }
  ]
}
```

For API-only examples:
```json
{
  "rewrites": [
    { "source": "/(.*)", "destination": "/api" }
  ]
}
```

#### package.json

Key differences from origin examples:
- Removes `srvx` and `vite-plugin-srvx`
- Uses `vercel dev` for development
- Simplified build scripts
- Uses `hono/vercel` adapter (built into the `hono` package)

#### README.md

Each Vercel example README includes:
- A note explaining it's the Vercel-optimized version with a link back to the origin
- A "Deploy with Vercel" button for one-click deployment

Example header:
```markdown
> **Note:** This is the Vercel-optimized version of the [hello-world](../hello-world) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=...)
```

### Skipped Examples

The following example types are not converted to Vercel:
- **Next.js examples** (`*-next-js`): Next.js has its own Vercel integration
- **Cloudflare examples** (`*-cloudflare*`): Different runtime environment
- **Deno examples**: Different runtime environment
- **Examples without `src/server.ts`**: Cannot be converted

### Workflow

1. Make changes to an origin example (e.g., `hello-world`)
2. Run the generation script to update the Vercel version
3. The script detects changes via git diff and only regenerates modified examples
4. Commit both the origin and generated Vercel examples

## TODO: Examples Cleanup

The following issues need to be fixed across examples:

- [x] Rename `src/registry.ts` to `src/actors.ts` in all examples
- [ ] Update all relative imports to use `.ts` extensions (ESM compliance) - only cloudflare examples remaining
- [ ] Add `allowImportingTsExtensions` and `rewriteRelativeImportExtensions` to tsconfig.json
- [x] Remove unused `tsup.config.ts` from examples using vite-plugin-srvx
- [x] Remove unused `tsup` devDependency from examples using vite-plugin-srvx
- [x] Move `srvx` from devDependencies to dependencies (used by `start` script)
- [x] Move `@hono/node-server` and `@hono/node-ws` from devDependencies to dependencies
- [x] Remove unused `concurrently` devDependency from examples using vite-plugin-srvx
- [ ] Remove `scripts/` directories with CLI client scripts - only cloudflare/next-js examples remaining
- [x] Remove `prompts` and `@types/prompts` devDependencies
- [x] Migrate all frontend examples to use vite-plugin-srvx
