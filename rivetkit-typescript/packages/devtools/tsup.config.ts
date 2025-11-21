import { defineConfig } from "tsup";
import defaultConfig from "../../../tsup.base.ts";

export default defineConfig({
	...defaultConfig,
	loader: {
		".svg": "dataurl",
		".css": "text",
	},
});
