import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { registry } from "../src/actors/index.ts";

describe("simple actor without Effect wrappers", () => {
	test("getValue and setValue work", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const simple = client.simple.getOrCreate(["test-simple"]);

		// Initial value should be 0
		const initialValue = await simple.getValue();
		expect(initialValue).toBe(0);

		// Set to 42
		const newValue = await simple.setValue(42);
		expect(newValue).toBe(42);

		// Get should return 42
		const finalValue = await simple.getValue();
		expect(finalValue).toBe(42);
	});
});
