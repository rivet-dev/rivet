# Example Project Structure

This document explains why RivetKit examples are structured the way they are. The goal is a single project structure that works without extra configuration across:

- Vercel
- Node (for the majority of "normal" deployments like Kubernetes, Railway, etc.)
- Bun & Deno (for hipster devs)

See `examples/CLAUDE.md` for the specific structure and patterns to follow.

## Platform Constraints

### Vercel

- Expects entry point at `src/server.ts` (not configurable)
- Static files must be in `public/` directory - served automatically via CDN
- Requires strict ESM imports using `.ts` suffix
- Requires the framework to be set to Hono to enable Vercel's "magic" WinterTC support
- Auto-detects Vite when it sees `vite.config.ts` and ignores Hono, so `vercel.json` must explicitly set `"framework": "hono"`

Reference: https://vercel.com/docs/frameworks/backend/hono

### Node

- No native TypeScript support
- No native WinterCG/WinterTC APIs
- No built-in static file serving
- WebSocket support requires `@hono/node-server` and `@hono/node-ws` adapters

### Bun & Deno

- No built-in static file serving

Everything else tends to _just work_ since these are modern ESM-compliant and WinterTC-compliant platforms.

## Structure

### src/server.ts with WinterTC

The server entry point must be `src/server.ts` with a default export. **You must explicitly import from `"hono"` for Vercel to detect the framework.**

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

- **Vercel** uses the default export to create serverless functions (requires `import { Hono } from "hono"` to detect framework)
- **WinterTC**-compatible runtimes expect this pattern
- **srvx** wraps the export for Node.js
- **Bun/Deno** can run it directly, srvx delegates to Bun/Deno if detected

### vercel.json

Vercel auto-detects Vite when it sees a `vite.config.ts` and ignores Hono. We must explicitly set the framework:

```json
{
  "framework": "hono"
}
```

Without this, Vercel won't enable WinterTC support and the server won't work correctly.

### public/ for Static Files

Static files are served from the `public/` directory:

- **Vercel** serves `public/` automatically via CDN with caching headers
- **srvx** serves files with `--static=public/`, matching Vercel's behavior
- **Bun/Deno** rely on srvx for the static file serving

This avoids platform-specific static file handling code.

### srvx + Vite + vite-plugin-srvx

For Node.js, we use `srvx` to bridge the gaps:

- Provides automatic TypeScript compilation
- Provides WinterTC compatibility shims
- `--static=public/` for static file serving

**Development** (with Vite):

```typescript
// vite.config.ts
import srvx from "vite-plugin-srvx";

export default defineConfig({
  plugins: [react(), ...srvx({ entry: "src/server.ts" })],
});
```

`vite-plugin-srvx` provides a unified dev server with frontend HMR and backend API handling.

**Production**:

```bash
srvx --static=public/ dist/server.js
```

Serves both the API and static files, matching Vercel's behavior.
