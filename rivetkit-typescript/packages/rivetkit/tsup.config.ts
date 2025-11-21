/// <reference types="@types/node" />

import { defineConfig } from "tsup";
import defaultConfig from "../../../tsup.base.ts";

export default defineConfig({
	...defaultConfig,
	outDir: "dist/tsup/",
	define: {
		"globalThis.CUSTOM_RIVETKIT_DEVTOOLS_URL": process.env
			.CUSTOM_RIVETKIT_DEVTOOLS_URL
			? `"${process.env.CUSTOM_RIVETKIT_DEVTOOLS_URL}"`
			: "false",
	},
});
