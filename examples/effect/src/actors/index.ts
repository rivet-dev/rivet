import { setup } from "rivetkit";
import { fetchActor } from "./fetch-actor.ts";
import { queueProcessor } from "./queue-processor.ts";

export { fetchActor } from "./fetch-actor.ts";
export { queueProcessor } from "./queue-processor.ts";

export const registry = setup({
	use: { fetchActor, queueProcessor },
});

export type Registry = typeof registry;
