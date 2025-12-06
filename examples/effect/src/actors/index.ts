import { setup } from "rivetkit";
import { counter } from "./counter";
import { user } from "./user";

export const registry = setup({
	use: { counter, user },
});

export type Registry = typeof registry;
