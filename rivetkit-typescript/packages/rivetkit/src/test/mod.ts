import pRetry from "p-retry";
import type { TestContext } from "vitest";
import { type Client, createClient } from "@/client/mod";
import { convertRegistryConfigToClientConfig } from "@/client/config";
import { getMetadata } from "@/engine-client/api-endpoints";
import type { Registry } from "@/registry";

export interface SetupTestResult<A extends Registry<any>> {
	client: Client<A>;
}

async function waitForRegistryReady(registry: Registry<any>): Promise<void> {
	const clientConfig = convertRegistryConfigToClientConfig(
		registry.parseConfig(),
	);

	await pRetry(async () => {
		await getMetadata(clientConfig);
	}, {
		retries: 20,
		minTimeout: 50,
		maxTimeout: 250,
	});
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
