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

### Actor File Structure

Actor definitions (`export const myActor = actor({...})`) must appear at the top of the file, before any helper functions. Helper functions, type definitions used only by helpers, and utilities go after the actor definition. This keeps the actor's public API front-and-center.

```typescript
// Good
export const myActor = actor({
  actions: {
    doThing: (c) => helperFunction(c),
  },
});

function helperFunction(c: ActorContextOf<typeof myActor>) {
  // ...
}

// Bad - don't put helpers above the actor
function helperFunction(...) { ... }

export const myActor = actor({...});
```

Shared types/interfaces used by both the actor definition and helpers (e.g. `State`, `PlayerEntry`) should go above the actor since the actor definition depends on them.

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
import app from "../src/server.ts";

export default app;
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

## Frontend Style Guide

Examples should follow these design conventions:

**Color Palette (Dark Theme)**
- Primary accent: `#ff4f00` (orange) for interactive elements and highlights
- Background: `#000000` (main), `#1c1c1e` (cards/containers)
- Borders: `#2c2c2e`
- Input backgrounds: `#2c2c2e` with border `#3a3a3c`
- Text: `#ffffff` (primary), `#8e8e93` (secondary/muted)
- Success: `#30d158` (green)
- Warning: `#ff4f00` (orange)
- Danger: `#ff3b30` (red)
- Purple: `#bf5af2` (for special states like rollback)

**Typography**
- UI: System fonts (`-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Inter', Roboto, sans-serif`)
- Code: `ui-monospace, SFMono-Regular, 'SF Mono', Consolas, monospace`
- Sizes: 14-16px body, 12-13px labels, large numbers 48-72px

**Sizing & Spacing**
- Border radius: 8px (cards/containers/buttons), 6px (inputs/badges)
- Section padding: 20-24px
- Gap between items: 12px
- Transitions: 200ms ease for all interactive states

**Button Styles**
- Padding: 12px 20px
- Border: none
- Border radius: 8px
- Font size: 14px, weight 600
- Hover: none (no hover state)
- Disabled: 50% opacity, `cursor: not-allowed`

**CSS Approach**
- Plain CSS in `<style>` tag within index.html (no preprocessors or Tailwind)
- Class-based selectors with state modifiers (`.active`, `.complete`, `.running`)
- Focus states use primary accent color (`#ff4f00`) for borders with subtle box-shadow

**Spacing System**
- Base unit: 4px
- Scale: 4px, 8px, 12px, 16px, 20px, 24px, 32px, 48px
- Component internal padding: 12-16px
- Section/card padding: 20px
- Card header padding: 16px 20px
- Gap between related items: 8-12px
- Gap between sections: 24-32px
- Margin between major blocks: 32px

**Iconography**
- Icon library: [Lucide](https://lucide.dev/) (React: `lucide-react`)
- Standard sizes: 16px (inline/small), 20px (buttons/UI), 24px (standalone/headers)
- Icon color: inherit from parent text color, or use `currentColor`
- Icon-only buttons must include `aria-label` for accessibility
- Stroke width: 2px (default), 1.5px for smaller icons

**Component Patterns**

*Buttons*
- Primary: `#ff4f00` background, white text
- Secondary: `#2c2c2e` background, white text
- Ghost: transparent background, `#ff4f00` text
- Danger: `#ff3b30` background, white text
- Success: `#30d158` background, white text
- Disabled: 50% opacity, `cursor: not-allowed`

*Form Inputs*
- Background: `#2c2c2e`
- Border: 1px solid `#3a3a3c`
- Border radius: 8px
- Padding: 12px 16px
- Focus: border-color `#ff4f00`, box-shadow `0 0 0 3px rgba(255, 79, 0, 0.2)`
- Placeholder text: `#6e6e73`

*Cards/Containers*
- Background: `#1c1c1e`
- Border: 1px solid `#2c2c2e`
- Border radius: 8px
- Padding: 20px
- Box shadow: `0 1px 3px rgba(0, 0, 0, 0.3)`
- Header style (when applicable):
  - Background: `#2c2c2e`
  - Padding: 16px 20px
  - Font size: 18px, weight 600
  - Border bottom: 1px solid `#2c2c2e`
  - Border radius: 8px 8px 0 0 (top corners only)
  - Negative margin to align with card edges: `-20px -20px 20px -20px`

*Modals/Overlays*
- Backdrop: `rgba(0, 0, 0, 0.75)`
- Modal background: `#1c1c1e`
- Border radius: 8px
- Max-width: 480px (small), 640px (medium), 800px (large)
- Padding: 24px
- Close button: top-right, 8px from edges

*Lists*
- Item padding: 12px 16px
- Dividers: 1px solid `#2c2c2e`
- Hover background: `#2c2c2e`
- Selected/active background: `rgba(255, 79, 0, 0.15)`

*Badges/Tags*
- Padding: 4px 8px
- Border radius: 6px
- Font size: 12px
- Font weight: 500

*Tabs*
- Container: `border-bottom: 1px solid #2c2c2e`, flex-wrap for overflow
- Tab: `padding: 12px 16px`, no background, `border-radius: 0`
- Tab border: `border-bottom: 2px solid transparent`, `margin-bottom: -1px`
- Tab text: `#8e8e93` (muted), font-weight 600, font-size 14px
- Active tab: `color: #ffffff`, `border-bottom-color: #ff4f00`
- Hover: none (no hover state)
- Transition: `color 200ms ease, border-color 200ms ease`

**UI States**

*Loading States*
- Spinner: 20px for inline, 32px for page-level
- Skeleton placeholders: `#2c2c2e` background with subtle pulse animation
- Loading text: "Loading..." in muted color
- Button loading: show spinner, disable interaction, keep button width stable

*Empty States*
- Center content vertically and horizontally
- Icon: 48px, muted color (`#6e6e73`)
- Heading: 18px, primary text color
- Description: 14px, muted color
- Optional action button below description

*Error States*
- Inline errors: `#ff3b30` text below input, 12px font size
- Error banners: `#ff3b30` left border (4px), `rgba(255, 59, 48, 0.1)` background
- Form validation: highlight input border in `#ff3b30`
- Error icon: Lucide `AlertCircle` or `XCircle`

*Disabled States*
- Opacity: 50%
- Cursor: `not-allowed`
- No hover/focus effects
- Preserve layout (don't collapse or hide)

*Success States*
- Color: `#30d158`
- Icon: Lucide `CheckCircle` or `Check`
- Toast/banner: `rgba(48, 209, 88, 0.1)` background with green left border

