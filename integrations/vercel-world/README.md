# `@rivet-dev/vercel-world`

[Vercel Workflows](https://useworkflow.dev) backed by Rivet Actors.

This integration is in beta.

Set `WORKFLOW_TARGET_WORLD=@rivet-dev/vercel-world`, then set
`WORKFLOW_RUNTIME_URL` and the Rivet connection variables in the application
environment. The package exports the standard `createWorld()` factory expected
by Vercel Eve and Vercel Workflows. The first World operation starts the
registry lazily and waits until its envoy is ready.

Applications that need to add actors to the same process can compose the
package's actor aggregate instead:

```ts
import { createWorld as createRivetWorld } from "@rivet-dev/vercel-world";
import { vercelWorldActors } from "@rivet-dev/vercel-world/registry";
import { setup } from "rivetkit";

import { myActor } from "./my-actor";

export const registry = setup({
	use: { ...vercelWorldActors, myActor },
});

export const createWorld = () => createRivetWorld({ registry });
```

This package only implements orchestration. Sandbox selection, including
agentOS, stays in Vercel Eve's sandbox configuration.
