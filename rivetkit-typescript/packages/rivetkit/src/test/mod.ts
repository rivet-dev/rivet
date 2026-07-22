import type { TestContext } from "vitest";
import { convertRegistryConfigToClientConfig } from "@/client/config";
import { type Client, createClient } from "@/client/mod";
import type { Registry } from "@/registry";
import { waitForRegistryReady } from "./ready";

export interface SetupTestResult<A extends Registry<any>> {
	client: Client<A>;
}

export async function setupTest<A extends Registry<any>>(
	c: TestContext,
	registry: A,
): Promise<SetupTestResult<A>> {
	registry.config.test = { ...registry.config.test, enabled: true };
	registry.config.noWelcome = true;

	registry.start();
	await waitForRegistryReady(registry);

	const client = createClient<A>({
		...convertRegistryConfigToClientConfig(registry.parseConfig()),
		disableMetadataLookup: false,
	});

	c.onTestFinished(async () => {
		await client.dispose();
	});

	return { client };
}
