import { defineConfig } from "tsup";

export default defineConfig({
	target: "es2020",
	format: ["cjs", "esm"],
	sourcemap: true,
	clean: true,
	dts: true,
	// Bundle only local dependencies
	noExternal: [/@rivet-gg\/.*?/],
	minify: true,
	platform: "neutral",
});
