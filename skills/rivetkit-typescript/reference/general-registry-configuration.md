# Registry Configuration

> Source: `src/content/docs/general/registry-configuration.mdx`
> Canonical URL: https://rivet.gg/docs/general/registry-configuration
> Description: This page documents the configuration options available when setting up a RivetKit registry. The registry configuration is passed to the `setup()` function.

---
## Example Configurations

### Basic Setup

```typescript
import { setup, actor } from "rivetkit";

const myActor = actor({ state: {}, actions: {} });

const registry = setup({
  use: { myActor },
});
```

### Connecting to Rivet Engine

```typescript {{"title":"Environment Variables"}}
import { setup, actor } from "rivetkit";

const myActor = actor({ state: {}, actions: {} });

// Reads from RIVET_ENDPOINT, RIVET_TOKEN, and RIVET_NAMESPACE
const registry = setup({
  use: { myActor },
});
```

```typescript {{"title":"Config"}}
import { setup, actor } from "rivetkit";

const myActor = actor({ state: {}, actions: {} });

const registry = setup({
  use: { myActor },
  endpoint: "https://api.rivet.dev",
  token: process.env.RIVET_TOKEN,
  namespace: "production",
});
```

## Starting Your App

After configuring your registry, start it using one of two runtime modes:

```typescript {{"title":"Serverless"}}
import { actor, setup } from "rivetkit";

const myActor = actor({ state: {}, actions: {} });
const registry = setup({ use: { myActor } });

export default registry.serve();
```

```typescript {{"title":"Serverless with Router"}}
import { Hono } from "hono";
import { actor, setup } from "rivetkit";

const myActor = actor({ state: {}, actions: {} });
const registry = setup({ use: { myActor } });

const app = new Hono();
app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

export default app;
```

```typescript {{"title":"Runner"}}
import { actor, setup } from "rivetkit";

const myActor = actor({ state: {}, actions: {} });
const registry = setup({ use: { myActor } });

registry.startRunner();
```

See [Runtime Modes](/docs/general/runtime-modes) for details on when to use each mode.

## Environment Variables

Many configuration options can be set via environment variables. See [Environment Variables](/docs/general/environment-variables) for a complete reference.

## Configuration Reference

## Related

- [Actor Configuration](/docs/general/actor-configuration)- Configure individual actors
- [HTTP Server Setup](/docs/general/http-server): Set up HTTP routing and middleware
- [Architecture](/docs/general/architecture): Understand how RivetKit works

_Source doc path: /docs/general/registry-configuration_
