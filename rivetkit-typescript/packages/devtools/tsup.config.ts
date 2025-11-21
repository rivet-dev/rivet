import { defineConfig } from "tsup";
import defaultConfig from "../../../tsup.base.ts";

export default defineConfig({
	...defaultConfig,
	entry: {
		"src/mod.tsx": "mod.js",
	},
	outExtension: () => {
		return {
			js: ".js",
		};
	},
	platform: "browser",
	dts: false,
	bundle: true,
	format: "iife",
	loader: {
		".svg": "dataurl",
		".css": "text",
	},
});
