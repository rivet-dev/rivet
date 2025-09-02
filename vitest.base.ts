import type { ViteUserConfig } from "vitest/config";

export default {
	test: {
		testTimeout: 5_000,
		hookTimeout: 5_000,
		// Enable parallelism
		sequence: {
			// TODO: This breaks fake timers, unsure how to make tests run in parallel within the same file
			// concurrent: true,
		},
		env: {
			// Enable logging
			_LOG_LEVEL: "DEBUG",
			_LOG_TARGET: "1",
			_LOG_TIMESTAMP: "1",
			_RIVETKIT_ERROR_STACK: "1",
		},
	},
} satisfies ViteUserConfig;
