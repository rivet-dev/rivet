import { setupTest } from "rivetkit/test";
import { expect, test } from "vitest";
import { registry } from "./counter";

// Replaces runInDurableObject() from @cloudflare/vitest-pool-workers
test("increments the counter", async (testCtx) => {
	const { client } = await setupTest(testCtx, registry);
	const counter = client.counter.getOrCreate(["test"]);

	await counter.increment(2);
	expect(await counter.increment(3)).toBe(5);
});
