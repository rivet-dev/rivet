import { setup } from "rivetkit";
import { port, useRivetCloud, warnIfRivetUsesGlobalHost } from "../config/env";
import { counter } from "./counter";

warnIfRivetUsesGlobalHost();

export const registry = setup(
	useRivetCloud
		? { use: { counter } }
		: {
				use: { counter },
				publicDir: "public",
				managerPort: port,
			},
);

if (!useRivetCloud) {
	registry.start();
}

/** Shared with the React app for `createRivetKit` typing. */
export type AppRegistry = typeof registry;
