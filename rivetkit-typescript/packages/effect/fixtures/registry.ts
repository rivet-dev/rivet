import { setup } from "rivetkit";
import { counter } from "./counter.ts";
import { simple } from "./simple.ts";

export const registry = setup({
	use: { counter, simple },
});
