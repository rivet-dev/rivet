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
			LOG_LEVEL: "DEBUG",
			LOG_TARGET: "1",
			LOG_TIMESTAMP: "1",
			_RIVETKIT_ERROR_STACK: "1",
			_RIVETKIT_LOG_MESSAGE: "1",
		},
	},
} satisfies ViteUserConfig;
