import invariant from "invariant";
import { type TestContext } from "vitest";
import { type Client, createClient } from "@/client/mod";
import { type Registry } from "@/mod";
import { Runtime } from "../../runtime";

export interface SetupTestResult<A extends Registry<any>> {
	client: Client<A>;
}

// Must use `TestContext` since global hooks do not work when running concurrently
export async function setupTest<A extends Registry<any>>(
	c: TestContext,
	registry: A,
): Promise<SetupTestResult<A>> {
	registry.config.test = { ...registry.config.test, enabled: true };
	registry.config.serveManager = true;
	registry.config.managerPort = 10_000 + Math.floor(Math.random() * 40_000);
	registry.config.inspector = {
		enabled: true,
		token: () => "token",
	};

	const runtime = await Runtime.create(registry);
	await runtime.startEnvoy();
	await new Promise((resolve) => setTimeout(resolve, 250));

	invariant(runtime.managerPort, "missing runtime manager port");
	const endpoint = `http://127.0.0.1:${runtime.managerPort}`;

	const client = createClient<A>({
		endpoint,
		namespace: "default",
		poolName: "default",
		disableMetadataLookup: true,
	});

	c.onTestFinished(async () => {
		await client.dispose();
	});

	return { client };
}
