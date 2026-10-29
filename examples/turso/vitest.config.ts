import { defineConfig } from "vitest/config";

export default defineConfig({
	test: {
		include: ["tests/**/*.test.ts"],
		// Creating and deleting Turso databases takes a few seconds.
		testTimeout: 60_000,
	},
});
