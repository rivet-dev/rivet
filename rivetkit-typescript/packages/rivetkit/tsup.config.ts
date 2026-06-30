/// <reference types="@types/node" />

import { defineConfig } from "tsup";
import defaultConfig from "../../../tsup.base.ts";

export default defineConfig({
	...defaultConfig,
	outDir: "dist/tsup/",
	// Keep Node shims enabled for the Node package build. The CommonJS output
	// needs `import.meta.url` rewritten for `require("rivetkit")` consumers.
	// Browser entrypoints are built separately by tsup.browser.config.ts with
	// shims disabled.
	shims: true,
	esbuildOptions(options) {
		// Mark @rivetkit workspace packages as external to preserve their dependency chains
		options.external = [
			...(options.external || []),
			"@rivetkit/traces",
			"@rivetkit/traces/encoding",
			"@rivetkit/traces/otlp",
			"@rivetkit/workflow-engine",
			"@rivet-dev/agent-os-core",
		];
	},
	define: {
		"globalThis.CUSTOM_RIVETKIT_DEVTOOLS_URL": process.env
			.CUSTOM_RIVETKIT_DEVTOOLS_URL
			? `"${process.env.CUSTOM_RIVETKIT_DEVTOOLS_URL}"`
			: "false",
	},
});
