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
	esbuildOptions(options) {
		// Mark @rivetkit workspace packages as external to preserve their dependency chains
		options.external = [
			...(options.external || []),
			"@rivetkit/traces",
			"@rivetkit/traces/encoding",
			"@rivetkit/traces/otlp",
			"@rivetkit/workflow-engine",
			"@rivetkit/sqlite",
			"@rivetkit/sqlite-vfs",
		];
	},
	define: {
		"globalThis.CUSTOM_RIVETKIT_DEVTOOLS_URL": process.env
			.CUSTOM_RIVETKIT_DEVTOOLS_URL
			? `"${process.env.CUSTOM_RIVETKIT_DEVTOOLS_URL}"`
			: "false",
	},
});
