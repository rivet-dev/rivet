import { defineConfig } from "vitest/config";

export default defineConfig({
	test: {
		globals: true,
		environment: "node",
		testTimeout: 120000,
		hookTimeout: 120000,
	},
});
