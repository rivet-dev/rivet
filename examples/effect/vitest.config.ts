import { defineConfig } from "vitest/config";

export default defineConfig({
	server: {
		port: 5173,
	},
	test: {
		include: ["tests/**/*.test.ts"],
	},
});
