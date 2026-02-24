import { type TestContext, vi } from "vitest";
import { assertUnreachable } from "@/actor/utils";
import { type Client, createClient } from "@/client/mod";
import { createClientWithDriver } from "@/mod";
import type { registry } from "../../fixtures/driver-test-suite/registry";
import { logger } from "./log";
import type { DriverTestConfig } from "./mod";
import { createTestInlineClientDriver } from "./test-inline-client-driver";
import { ClientConfigSchema } from "@/client/config";

export const FAKE_TIME = new Date("2024-01-01T00:00:00.000Z");

// Must use `TestContext` since global hooks do not work when running concurrently
export async function setupDriverTest(
	c: TestContext,
	driverTestConfig: DriverTestConfig,
): Promise<{
	client: Client<typeof registry>;
	endpoint: string;
}> {
	if (!driverTestConfig.useRealTimers) {
		vi.useFakeTimers();
		vi.setSystemTime(FAKE_TIME);
	}

	// Build drivers
	const { endpoint, namespace, runnerName, cleanup } =
		await driverTestConfig.start();

	let client: Client<typeof registry>;
	if (driverTestConfig.clientType === "http") {
		// Create client
		client = createClient<typeof registry>({
			endpoint,
			namespace,
			runnerName,
			encoding: driverTestConfig.encoding,
			// Disable metadata lookup to prevent redirect to the wrong port.
			// Each test starts a new server on a dynamic port, but the
			// registry's publicEndpoint defaults to port 6420.
			disableMetadataLookup: true,
		});
	} else if (driverTestConfig.clientType === "inline") {
		// Use inline client from driver
		const encoding = driverTestConfig.encoding ?? "bare";
		const managerDriver = createTestInlineClientDriver(endpoint, encoding);
		const runConfig = ClientConfigSchema.parse({
			encoding: encoding,
		});
		client = createClientWithDriver(managerDriver, runConfig);
	} else {
		assertUnreachable(driverTestConfig.clientType);
	}

	c.onTestFinished(async () => {
		if (!driverTestConfig.HACK_skipCleanupNet) {
			await client.dispose();
		}

		logger().info("cleaning up test");
		await cleanup();
	});

	return {
		client,
		endpoint,
	};
}

export async function waitFor(
	driverTestConfig: DriverTestConfig,
	ms: number,
): Promise<void> {
	if (driverTestConfig.useRealTimers) {
		return new Promise((resolve) => setTimeout(resolve, ms));
	} else {
		vi.advanceTimersByTime(ms);
		return Promise.resolve();
	}
}
