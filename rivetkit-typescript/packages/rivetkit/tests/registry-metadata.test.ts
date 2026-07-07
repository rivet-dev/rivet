import { describe, expect, test } from "vitest";
import { actor } from "@/actor/definition";
import { buildActorNames, RegistryConfigSchema } from "@/registry/config";

describe("registry metadata", () => {
	test("does not request legacy actor KV preload metadata", () => {
		const config = RegistryConfigSchema.parse({
			use: {
				test: actor({
					state: {},
					actions: {},
				}),
			},
		});

		expect(buildActorNames(config).test.metadata).not.toHaveProperty(
			"preload",
		);
	});
});
