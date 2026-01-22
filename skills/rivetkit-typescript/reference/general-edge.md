# Edge Networking

> Source: `src/content/docs/general/edge.mdx`
> Canonical URL: https://rivet.gg/docs/general/edge
> Description: Actors automatically run near your users on your provider's global network.

---
At the moment, edge networking is only supported on Rivet Cloud & Cloudflare Workers. More self-hosted platforms are on the roadmap.

## Region selection

### Automatic region selection

By default, actors will choose the nearest region based on the client's location.

Under the hood, Rivet and Cloudflare use [Anycast routing](https://en.wikipedia.org/wiki/Anycast) to automatically find the best location for the client to connect to without relying on a slow manual pinging process.

### Manual region selection

The region an actor is created in can be overridden using region options:

```typescript client.ts
import { createClient } from "rivetkit/client";
import { actor, setup } from "rivetkit";

const example = actor({ state: {}, actions: {} });
const registry = setup({ use: { example } });

const client = createClient<typeof registry>();

// Create actor in a specific region using getOrCreate
const actorHandle = client.example.getOrCreate(["my-actor"], {
  createInRegion: "atl"
});
```

See [Create  Manage Actors](/docs/actors/communicating-between-actors) for more information.

_Source doc path: /docs/general/edge_
