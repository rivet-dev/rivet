# @rivet-dev/flue-target

A [Flue](https://github.com/rivet-dev/flue) target that compiles agents and
workflows into a [Rivet](https://rivet.dev) Actor application.

```sh
npm add @rivet-dev/flue-target
```

```ts
// flue.config.ts
import { defineConfig } from "@flue/cli/config";
import { rivet } from "@rivet-dev/flue-target";

export default defineConfig({
	target: rivet({ actors: "./actors.ts" }),
});
```

Export a RivetKit registry from `actors.ts`. `flue build` then emits a server
entrypoint that maps every agent and workflow to a durable Rivet Actor.

Full documentation: **https://rivet.dev/docs/integrations/flue**

Runnable example: [`examples/rivet`](./examples/rivet)
