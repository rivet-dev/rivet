import { defineConfig } from "vitest/config";
import defaultConfig from "../../../vitest.base.ts";

export default defineConfig({
	...defaultConfig,
	test: {
		...defaultConfig.test,
		include: ["tests/**/*.test.ts"],
	},
});
