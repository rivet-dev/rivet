import { join } from "node:path";
import { createTestRuntime, runDriverTests } from "@/driver-test-suite/mod";
import { createFileSystemOrMemoryDriver } from "@/drivers/file-system/mod";

runDriverTests({
	// TODO: Remove this once timer issues are fixed in actor-sleep.ts
	useRealTimers: true,
	async start() {
		return await createTestRuntime(
			join(__dirname, "../fixtures/driver-test-suite/registry.ts"),
			async () => {
				return {
					driver: await createFileSystemOrMemoryDriver(
						true,
						`/tmp/test-${crypto.randomUUID()}`,
					),
				};
			},
		);
	},
});
