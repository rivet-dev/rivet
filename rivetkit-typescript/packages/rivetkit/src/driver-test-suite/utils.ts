import invariant from "invariant";
import { type TestContext, vi } from "vitest";
import { assertUnreachable } from "@/actor/utils";
import { type Client, createClient } from "@/client/mod";
import { RunConfigSchema } from "@/driver-helpers/mod";
import { createClientWithDriver } from "@/mod";
import type { registry } from "../../fixtures/driver-test-suite/registry";
import type { DriverTestConfig } from "./mod";
import { createTestInlineClientDriver } from "./test-inline-client-driver";

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
	c.onTestFinished(cleanup);

	let client: Client<typeof registry>;
	if (driverTestConfig.clientType === "http") {
		// Create client
		client = createClient<typeof registry>({
			endpoint,
			namespace,
			runnerName,
			encoding: driverTestConfig.encoding,
		});
	} else if (driverTestConfig.clientType === "inline") {
		// Use inline client from driver
		const encoding = driverTestConfig.encoding ?? "bare";
		const managerDriver = createTestInlineClientDriver(endpoint, encoding);
		const runConfig = RunConfigSchema.parse({
			encoding: encoding,
		});
		client = createClientWithDriver(managerDriver, runConfig);
	} else {
		assertUnreachable(driverTestConfig.clientType);
	}

	// Cleanup client
	if (!driverTestConfig.HACK_skipCleanupNet) {
		c.onTestFinished(async () => await client.dispose());
	}

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
