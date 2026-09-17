import { afterEach, describe, expect, test } from "vitest";
import { RegistryConfigSchema } from "./index";

const ORIGINAL_RIVET_TOKEN = process.env.RIVET_TOKEN;

afterEach(() => {
	if (ORIGINAL_RIVET_TOKEN === undefined) {
		delete process.env.RIVET_TOKEN;
	} else {
		process.env.RIVET_TOKEN = ORIGINAL_RIVET_TOKEN;
	}
});

describe("RegistryConfigSchema", () => {
	test("defaults token to dev", () => {
		delete process.env.RIVET_TOKEN;

		const config = RegistryConfigSchema.parse({
			use: {},
			startEngine: true,
		});

		expect(config.token).toBe("dev");
	});
});
