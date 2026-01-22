# Deploying to Cloudflare Workers

> Source: `src/content/docs/connect/cloudflare-workers.mdx`
> Canonical URL: https://rivet.gg/docs/connect/cloudflare-workers
> Description: Deploy your Cloudflare Workers + RivetKit app to Cloudflare Workers.

---
Deploy your Cloudflare Workers + RivetKit app to [Cloudflare Workers](https://workers.cloudflare.com/).

- [Cloudflare Workers](https://github.com/rivet-dev/rivet/tree/main/examples/cloudflare-workers) — Minimal Cloudflare Workers + RivetKit example.

- [Cloudflare Workers + Hono](https://github.com/rivet-dev/rivet/tree/main/examples/cloudflare-workers-hono) — Cloudflare Workers with Hono router.

- [Cloudflare Workers + Inline Client](https://github.com/rivet-dev/rivet/tree/main/examples/cloudflare-workers-inline-client) — Advanced setup using createInlineClient.

## Steps

### Prerequisites

- [Cloudflare account](https://dash.cloudflare.com/) with Durable Objects enabled
- [Wrangler CLI](https://developers.cloudflare.com/workers/wrangler/install-and-update/) v3
- A Cloudflare Worker app integrated with RivetKit
  - See the [Cloudflare Workers quickstart](/docs/actors/quickstart/cloudflare-workers/) or [Cloudflare Workers example](https://github.com/rivet-dev/rivet/tree/main/examples/cloudflare-workers) to get started
- Access to the [Rivet Cloud](https://dashboard.rivet.dev/) or a [self-hosted Rivet Engine](/docs/general/self-hosting)

### Verify RivetKit integration with Cloudflare Workers

Your project should have the following files:

- `src/index.ts` (or similar entry point with `createHandler`)
- `src/registry.ts` (or similar actor registry file)
- `wrangler.json` with proper Durable Objects and KV namespace configuration

If your project is not integrated with RivetKit yet, follow the [Cloudflare Workers quickstart guide](/docs/actors/quickstart/cloudflare-workers/) or see the [Cloudflare Workers example](https://github.com/rivet-dev/rivet/tree/main/examples/cloudflare-workers).

### Deploy to Cloudflare Workers

Deploy to Cloudflare's global network:

```sh
wrangler deploy
```

Your worker will be deployed and you'll receive a URL like `https://my-rivetkit-worker.workers.dev`.

More information on deployments is available in [Cloudflare's docs](https://developers.cloudflare.com/workers/wrangler/commands/#deploy).

### Connect and Verify

After running `wrangler deploy`, note the URL printed in the output (e.g., `https://my-rivetkit-worker.workers.dev`).

Your RivetKit endpoint will be available at this URL with `/rivet` appended:
- Example: `https://my-rivetkit-worker.workers.dev/rivet`

Use this endpoint URL when configuring your RivetKit client or connecting from the Rivet dashboard.

## Advanced

### Accessing Environment Bindings

You can access Cloudflare Workers environment bindings directly using the importable `env`:

```typescript @nocheck
import { env } from "cloudflare:workers";
import { actor } from "rivetkit";

// Access environment variables and secrets in top-level scope
const API_KEY = env.API_KEY;
const LOG_LEVEL = env.LOG_LEVEL || "info";

// Use bindings in your actor
const myActor = actor({
  state: { count: 0 },
  
  actions: {
    // Access KV, D1, or other bindings during request handling
    getFromKV: async (c, key: string) => {
      // Access additional KV namespaces defined in wrangler.json
      if (env.MY_CACHE_KV) {
        return await env.MY_CACHE_KV.get(key);
      }
    }
  }
});
```

### Driver Context

The Cloudflare Workers driver provides access to the Durable Object state and environment through the driver context in `createVars`.

```typescript @nocheck
import { actor, CreateVarsContext } from "rivetkit";
import type { DriverContext } from "@rivetkit/cloudflare-workers";

const myActor = actor({
  state: { count: 0 },

  // Save the Cloudflare driver context
  createVars: (ctx: CreateVarsContext, driver: DriverContext) => ({
    state: driver.state,
  }),
  
  actions: {
    // Example: Access Durable Object info (not recommended in practice)
    kvGet: (c, key: string) => {
      const doState = c.vars.state;
	  return await doState.storage.get(key)
    },
  }
});
```

The Cloudflare Workers driver context type is exported as `DriverContext` from `@rivetkit/cloudflare-workers`:

```typescript @nocheck
interface DriverContext {
  state: DurableObjectState;
}
```

While you have access to the Durable Object state, be cautious when directly modifying KV storage or alarms, as this may interfere with RivetKit's internal operations and potentially break actor functionality.

_Source doc path: /docs/connect/cloudflare-workers_
