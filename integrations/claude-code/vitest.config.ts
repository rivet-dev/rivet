import { defineConfig, mergeConfig } from "vitest/config";
import baseConfig from "../../vitest.base.ts";

export default mergeConfig(
	baseConfig,
	defineConfig({
		test: {
			testTimeout: 30_000,
			hookTimeout: 30_000,
			fileParallelism: false,
		},
	}),
);
