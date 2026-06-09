import { setup } from "rivetkit";
import { registry as staticRegistry } from "./registry-static";

// Worker-runtime mirror of the static driver fixture registry: the same actor
// definitions with every actor's user code running in a per-actor worker
// thread. Mutating the shared definition options is safe because each test
// runtime process loads exactly one registry fixture, and the bridge worker
// child re-imports this module, re-applying the same mutation.

const use = staticRegistry.config.use;
for (const definition of Object.values(use)) {
	definition.config.options.runtime = "worker";
}

export const registry = setup({
	use,
	worker: { module: import.meta.url },
});
