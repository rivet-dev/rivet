import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

const here = dirname(fileURLToPath(import.meta.url));

// Point the SDK at a locally-built debug engine. Override RIVET_ENGINE_BINARY
// to use a different binary.
const engineBinary =
	process.env.RIVET_ENGINE_BINARY ??
	join(here, "../../target/debug/rivet-engine");

export default defineConfig({
	test: {
		include: ["tests/**/*.test.ts"],
		env: {
			RIVET_ENGINE_BINARY: engineBinary,
			// Keep the engine + runtime logs quiet during the suite.
			RIVET_LOG_LEVEL: "SILENT",
		},
		// One rivet-engine is shared across all test files in this suite via
		// globalSetup. Each test prepares its own namespace + runner pool, so
		// envoy registrations from one test can't pollute another. We serialize
		// files because `Registry.test` registers an in-process envoy bound to
		// local ports.
		fileParallelism: false,
		sequence: { concurrent: false },
		globalSetup: ["./tests/global-setup.ts"],
		testTimeout: 30_000,
		hookTimeout: 30_000,
	},
});
