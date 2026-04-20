import { resolve } from "node:path";
import tsconfigPaths from "vite-tsconfig-paths";
import { defineConfig } from "vitest/config";
import defaultConfig from "../../../vitest.base.ts";

export default defineConfig({
	...defaultConfig,
	test: {
		...defaultConfig.test,
		fileParallelism: false,
		testTimeout: 30_000,
		hookTimeout: 30_000,
		minWorkers: 1,
		maxWorkers: 1,
		sequence: {
			...defaultConfig.test.sequence,
			concurrent: false,
		},
	},
	// Used to resolve "rivetkit" to "src/mod.ts" in the test fixtures
	plugins: [tsconfigPaths()],
	resolve: {
		alias: {
			"@": resolve(__dirname, "./src"),
			"rivetkit/errors": resolve(__dirname, "./src/actor/errors.ts"),
		},
	},
});
