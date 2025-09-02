import { resolve } from "path";
import { defineConfig } from "vitest/config";
import defaultConfig from "../../../vitest.base.ts";

export default defineConfig({
	...defaultConfig,
	resolve: {
		alias: {
			"@": resolve(__dirname, "./src"),
		},
	},
	test: {
		...defaultConfig.test,
		include: ["tests/**/*.test.ts"],
	},
});

