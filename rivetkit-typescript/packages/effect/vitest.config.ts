// <reference types="node" />
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";
import defaultConfig from "../../../vitest.base";

const here = dirname(fileURLToPath(import.meta.url));

const env = {
	...defaultConfig.test?.env,
	RIVET_ENGINE_BINARY: join(here, "../../../target/debug/rivet-engine"),
	// The shared vitest base sets RIVET_LOG_LEVEL=DEBUG, which floods the
	// terminal with engine + runtime logs. Keep this suite quiet.
	RIVET_LOG_LEVEL: "SILENT",
};

export default defineConfig({
	...defaultConfig,
	test: {
		...defaultConfig.test,
		env,
		// One rivet-engine is shared across all test files in this suite.
		// Each file creates its own namespace + runner pool against it, so
		// envoy registrations from one file can't pollute another. We
		// still serialize files for now because `Registry.test` registers
		// an in-process envoy that binds local ports.
		fileParallelism: false,
		sequence: { concurrent: false },
		globalSetup: ["./test/global-setup.ts"],
		coverage: {
			include: ["src/**/*.ts"],
			exclude: ["*.test-d.ts"],
		},
	},
});
