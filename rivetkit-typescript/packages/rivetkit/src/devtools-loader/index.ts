import type { ClientConfigInput } from "@/client/client";
import { VERSION } from "@/utils";
import { logger } from "./log";

declare global {
	// Injected via tsup config
	// biome-ignore lint/style/noVar: required for global declaration
	var CUSTOM_RIVETKIT_DEVTOOLS_URL: string | undefined;
}

const DEFAULT_DEVTOOLS_URL = (version = VERSION) =>
	`https://releases.rivet.dev/rivet/latest/devtools/mod.js?v=${version}`;

const scriptId = "rivetkit-devtools-script";

export function injectDevtools(config: ClientConfigInput) {
	if (!window) {
		logger().warn("devtools not available outside browser environment");
		return;
	}

	if (!document.getElementById(scriptId)) {
		const script = document.createElement("script");
		script.id = scriptId;
		script.src =
			globalThis.CUSTOM_RIVETKIT_DEVTOOLS_URL || DEFAULT_DEVTOOLS_URL();
		script.async = true;
		document.head.appendChild(script);
	}

	window.__rivetkit = window.__rivetkit || [];
	window.__rivetkit.push(config);
}
