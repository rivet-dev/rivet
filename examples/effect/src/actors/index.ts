import { setup } from "rivetkit";
import { counter } from "./counter.ts";
import { user } from "./user.ts";
import { lifecycleDemo } from "./lifecycle-demo.ts";
import { simple } from "./simple.ts";

// Re-export individual actors
export { counter } from "./counter.ts";
export { user } from "./user.ts";
export { lifecycleDemo } from "./lifecycle-demo.ts";
export { simple } from "./simple.ts";

// Registry setup with all actors
export const registry = setup({
	use: { counter, user, lifecycleDemo, simple },
});

export type Registry = typeof registry;
