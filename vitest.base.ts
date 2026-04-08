import { availableParallelism, cpus } from "node:os";
import type { ViteUserConfig } from "vitest/config";

const maxConcurrency = (() => {
	try {
		return availableParallelism();
	} catch {
		return cpus().length;
	}
})();

export default {
	test: {
		testTimeout: 10_000,
		hookTimeout: 10_000,
		maxConcurrency,
		// Enable parallelism
		sequence: {
			concurrent: true,
		},
		env: {
			// Enable logging
			RIVET_LOG_LEVEL: "DEBUG",
			RIVET_LOG_TARGET: "1",
			RIVET_LOG_TIMESTAMP: "1",
			RIVET_LOG_ERROR_STACK: "1",
			RIVET_LOG_MESSAGE: "1",
		},
	},
} satisfies ViteUserConfig;
