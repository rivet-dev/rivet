# `@rivet-dev/workflow-world`

Vercel's [Workflow SDK](https://workflow-sdk.dev) backed by Rivet Actors.

This integration is in beta.

Set `WORKFLOW_TARGET_WORLD=@rivet-dev/workflow-world` and `WORKFLOW_RUNTIME_URL`
in the application environment. The Rivet connection resolves through RivetKit's
[standard environment
variables](https://rivet.dev/docs/environment-variables), so local
development needs nothing further. Set them only when connecting to a remote
control plane.

The package exports the standard `createWorld()` factory expected by Vercel Eve
and the Workflow SDK. The first World operation starts the registry lazily and
waits until its envoy is ready.

Applications that need to add actors to the same process can compose the
package's actor aggregate instead:

```ts
import { createWorld as createRivetWorld } from "@rivet-dev/workflow-world";
import { workflowWorldActors } from "@rivet-dev/workflow-world/registry";
import { setup } from "rivetkit";

import { myActor } from "./my-actor";

export const registry = setup({
	use: { ...workflowWorldActors, myActor },
});

export const createWorld = () => createRivetWorld({ registry });
```

## Next.js

RivetKit loads its runtime through a dynamic import, so a bundler cannot resolve
it statically. Mark it external in `next.config.ts`:

```ts
import { withWorkflow } from "workflow/next";
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
	serverExternalPackages: ["rivetkit"],
};

export default withWorkflow(nextConfig);
```

Without it the flow route fails at request time with `Cannot find module`.

List `rivetkit` rather than `@rivet-dev/workflow-world`. The two need opposite
treatment: `withWorkflow` resolves the target World through a build alias and
compiles it into the server bundle, while `serverExternalPackages` means the
package is left to Node at runtime. Next.js rejects a package asked to do both.

## Scope

This package only implements orchestration. Sandbox selection, including
agentOS, stays in Vercel Eve's sandbox configuration.

## Workflow SDK compatibility

This package implements the Workflow SDK 5 World contract and requires
`workflow` 5.2 or newer. Event ids are positions in the run's log
(`evnt_00000000000000000000000001` is a run's first event), allocated inside
the run actor's transaction that appends the event.

Runs created by an earlier release of this package numbered their events with
ULIDs, and the Workflow SDK 5 runtime refuses to replay them. Every run is
served by the deployment at `WORKFLOW_RUNTIME_URL`, so let in-flight runs
finish on the previous release before upgrading.
