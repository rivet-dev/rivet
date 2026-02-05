/// <reference types="@types/node" />

import { defineConfig } from "tsup";
import defaultConfig from "../../../tsup.base.ts";

export default defineConfig({
	...defaultConfig,
	outDir: "dist/tsup/",
	// Override shims: false to prevent ESM shims (fileURLToPath, etc.) from being
	// injected into chunks. The shims import Node.js-only modules which break
	// browser builds when importing from rivetkit/client.
	// See: https://github.com/egoist/tsup/issues/958
	shims: false,
	define: {
		"globalThis.CUSTOM_RIVETKIT_DEVTOOLS_URL": process.env
			.CUSTOM_RIVETKIT_DEVTOOLS_URL
			? `"${process.env.CUSTOM_RIVETKIT_DEVTOOLS_URL}"`
			: "false",
	},
});
