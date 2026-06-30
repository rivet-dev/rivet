import { describe, expect, test } from "vitest";
import { resolveServerlessDrainGracePeriod } from "./configure";

describe("resolveServerlessDrainGracePeriod", () => {
	test("preserves the engine default for the default request lifespan", () => {
		expect(resolveServerlessDrainGracePeriod(60 * 60, undefined)).toBe(
			30 * 60,
		);
	});

	test("uses a valid short default for Next.js-style request lifespans", () => {
		expect(resolveServerlessDrainGracePeriod(300, undefined)).toBe(30);
	});

	test("keeps the generated default below very short request lifespans", () => {
		expect(resolveServerlessDrainGracePeriod(10, undefined)).toBe(9);
	});

	test("preserves explicit drain grace period configuration", () => {
		expect(resolveServerlessDrainGracePeriod(300, 5)).toBe(5);
	});
});
