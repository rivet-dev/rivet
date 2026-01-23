import { defineConfig } from "vitest/config";

export default defineConfig({
	test: {
		// Use edge-runtime to match Convex's default runtime.
		environment: "edge-runtime",
		// Inline convex-test for proper dependency resolution.
		server: { deps: { inline: ["convex-test"] } },
		// Run tests from tests/ directory.
		include: ["tests/**/*.test.ts"],
	},
});
