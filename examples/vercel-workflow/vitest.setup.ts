import { registry } from "@rivet-dev/vercel-world/registry";
import { afterAll } from "vitest";

registry.config.shutdown = {
	disableSignalHandlers: true,
	gracePeriodMs: 1_000,
};
afterAll(async () => {
	await registry.shutdown();
});
