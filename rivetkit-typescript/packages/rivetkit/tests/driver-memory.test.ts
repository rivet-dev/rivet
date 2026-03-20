import { createTestRuntime, runDriverTests } from "@/driver-test-suite/mod";
import { createFileSystemOrMemoryDriver } from "@/drivers/file-system/mod";
import { describe } from "vitest";
import { getDriverRegistryVariants } from "./driver-registry-variants";

for (const registryVariant of getDriverRegistryVariants(__dirname)) {
	const describeVariant = registryVariant.skip
		? describe.skip
		: describe.sequential;
	const variantName = registryVariant.skipReason
		? `${registryVariant.name} (${registryVariant.skipReason})`
		: registryVariant.name;

	describeVariant(`registry (${variantName})`, () => {
		runDriverTests({
			// TODO: Remove this once timer issues are fixed in actor-sleep.ts
			useRealTimers: true,
			isDynamic: registryVariant.name === "dynamic",
			features: {
				hibernatableWebSocketProtocol: false,
			},
			skip: {
				// Sleeping not enabled in memory
				sleep: true,
				hibernation: true,
			},
			async start() {
				return await createTestRuntime(
					registryVariant.registryPath,
					async () => {
						return {
							driver: await createFileSystemOrMemoryDriver(false),
						};
					},
				);
			},
		});
	});
}
