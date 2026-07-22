import pRetry from "p-retry";
import type { Registry } from "@/registry";

export async function waitForRegistryReady(
	registry: Registry<any>,
): Promise<void> {
	await pRetry(
		async () => {
			const response = await registry.routes.health();
			if (!response.ok) {
				throw new Error(
					`RivetKit registry is not ready: ${response.status} ${await response.text()}`,
				);
			}
		},
		{
			retries: 120,
			minTimeout: 50,
			maxTimeout: 250,
		},
	);
}
