import { resolve } from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
	resolve: {
		alias: {
			"@rivetkit/framework-base": resolve(
				__dirname,
				"../framework-base/src/mod.ts",
			),
		},
	},
	test: {
		environment: "jsdom",
		globals: false,
		testTimeout: 10_000,
	},
});
