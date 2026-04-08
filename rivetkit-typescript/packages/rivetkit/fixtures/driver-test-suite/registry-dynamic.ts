import { setup } from "rivetkit";
import type { registry as DriverTestRegistryType } from "./registry";
import { loadDynamicActors } from "./registry-loader";

const use = loadDynamicActors();

export const registry = setup({
	use,
}) as unknown as typeof DriverTestRegistryType;
