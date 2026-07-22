import { afterEach, expect, test, vi } from "vitest";
import type { Registry } from "@/registry";
import { waitForRegistryReady } from "@/test/ready";

afterEach(() => {
	vi.useRealTimers();
});

test("waits for the registry Engine connection instead of Engine metadata", async () => {
	vi.useFakeTimers();
	const health = vi
		.fn()
		.mockResolvedValueOnce(
			Response.json({ status: "starting" }, { status: 503 }),
		)
		.mockResolvedValueOnce(Response.json({ status: "ok" }));
	const registry = { routes: { health } } as unknown as Registry<any>;

	const ready = waitForRegistryReady(registry);
	await vi.runAllTimersAsync();
	await ready;

	expect(health).toHaveBeenCalledTimes(2);
});
