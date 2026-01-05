import { defineConfig } from "tsup";

export default defineConfig({
	entry: {
		server: "src/server.ts",
	},
	format: ["esm"],
	outDir: "dist",
	bundle: true,
	splitting: false,
	shims: true,
});
