/// <reference types="@types/node" />

import { defineConfig } from "tsup";

export default defineConfig({
	entry: ["dynamic-isolate-runtime/src/index.cts"],
	tsconfig: "dynamic-isolate-runtime/tsconfig.json",
	outDir: "dist/dynamic-isolate-runtime",
	format: ["cjs"],
	platform: "node",
	target: "node22",
	sourcemap: true,
	clean: false,
	dts: false,
	minify: false,
	splitting: false,
	noExternal: [/.*/],
	external: [/^node:.*/],
	outExtension() {
		return {
			js: ".cjs",
		};
	},
});
