import { defineConfig, mergeConfig } from "vitest/config";
import baseConfig from "../../vitest.base.ts";

export default mergeConfig(
	baseConfig,
	defineConfig({
		test: {
			// Each file boots its own engine through setupTest. Running files in
			// parallel makes those engines contend for ports and CPU.
			fileParallelism: false,
			sequence: { concurrent: false },
			testTimeout: 60_000,
			hookTimeout: 60_000,
			pool: "forks",
		},
	}),
);
