import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { registry } from "../src/actors.ts";

describe("Counter Actor", () => {
	test("fetch handler returns counter state", async (test) => {
		const { client } = await setupTest(test, registry);
		const handle = client.counter.getOrCreate("test-fetch");

		// GET current state
		let response = await handle.fetch("/count");
		expect(response.status).toBe(200);
		let data = await response.json();
		expect(data.count).toBe(0);

		// POST to increment
		response = await handle.fetch("/increment", { method: "POST" });
		expect(response.status).toBe(200);
		data = await response.json();
		expect(data.count).toBe(1);

		// Verify state persisted with another GET
		response = await handle.fetch("/count");
		data = await response.json();
		expect(data.count).toBe(1);
	});
});
