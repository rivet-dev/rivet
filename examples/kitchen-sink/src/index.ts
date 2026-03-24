import { setup } from "rivetkit";
import { demo } from "./actors/demo.ts";
import { sqliteBench } from "./actors/sqlite-bench.ts";

export const registry = setup({
	use: {
		demo,
		sqliteBench,
	},
});

export type Registry = typeof registry;

registry.start();
