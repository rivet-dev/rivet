import type { ViteUserConfig } from "vitest/config";

export default {
	test: {
		testTimeout: 10_000,
		hookTimeout: 10_000,
		// Enable parallelism
		sequence: {
			// TODO: This breaks fake timers, unsure how to make tests run in parallel within the same file
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
