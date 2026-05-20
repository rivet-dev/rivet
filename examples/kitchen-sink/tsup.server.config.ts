import { defineConfig } from "tsup";

export default defineConfig({
	entry: {
		server: "src/server.ts",
	},
	format: ["esm"],
	outDir: "dist-server",
	outExtension: () => ({ js: ".mjs" }),
	platform: "node",
	loader: {
		".sql": "text",
	},
	sourcemap: true,
	splitting: false,
	clean: true,
});
