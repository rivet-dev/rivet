import { setup } from "rivetkit";
import type { registry as DriverTestRegistryType } from "./registry";
import { loadStaticActors } from "./registry-loader";

const use = await loadStaticActors();

export const registry = setup({
	use,
}) as typeof DriverTestRegistryType;
