# @rivetkit/supabase

Supabase Edge Functions integration for [RivetKit](https://rivet.dev) actors.

Host Rivet Actors in a Supabase Edge Function (Deno) with a single import. The
wasm runtime and wasm binary loading are wired automatically.

```ts
import { actor } from "rivetkit";
import { serve } from "@rivetkit/supabase";

const counter = actor({
	state: { count: 0 },
	actions: {
		increment: (c, amount = 1) => (c.state.count += amount),
		getCount: (c) => c.state.count,
	},
});

await serve({ use: { counter } });
```

Set `RIVET_ENDPOINT` as a function secret (namespace and token may be embedded
in the URL as `https://namespace:token@host`).

## Mounting your own routes

Pass `fetch` to handle everything outside the Rivet manager API path:

```ts
await serve({ use: { counter } }, {
	fetch: (request) => {
		if (new URL(request.url).pathname.endsWith("/health")) {
			return new Response("ok");
		}
		return new Response("not found", { status: 404 });
	},
});
```

Learn more at https://rivet.dev/docs.
