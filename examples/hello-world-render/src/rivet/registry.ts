import { setup } from "rivetkit";
import { useRivetCloud, warnIfRivetUsesGlobalHost } from "../config/env";
import { counter } from "./counter";

warnIfRivetUsesGlobalHost();

export const registry = setup(
	useRivetCloud
		? { use: { counter } }
		: {
				use: { counter },
			},
);

if (!useRivetCloud) {
	registry.start();
}

/** Shared with the React app for `createRivetKit` typing. */
export type AppRegistry = typeof registry;
