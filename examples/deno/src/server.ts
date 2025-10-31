import { upgradeWebSocket } from "hono/deno";
import { registry } from "./registry.ts";

const { fetch } = registry.start({
	// Deno requires using Deno.serve
	disableDefaultServer: true,
	overrideServerAddress: "http://localhost:6420",
	// Specify Deno-specific upgradeWebSocket
	getUpgradeWebSocket: () => upgradeWebSocket,
});

// Start server
Deno.serve({ port: 8080 }, fetch);
