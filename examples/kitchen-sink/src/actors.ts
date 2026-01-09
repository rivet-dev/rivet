import { setup } from "rivetkit";
import { demo } from "./actors/demo.ts";

export const registry = setup({
	use: {
		demo,
	},
});

export type Registry = typeof registry;
