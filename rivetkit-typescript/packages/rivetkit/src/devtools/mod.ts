/// <reference lib="dom" />

declare global {
	interface Window {
		__rivetkit?: ClientConfigInput[];
	}
	// injected via tsup config
	var CUSTOM_RIVETKIT_DEVTOOLS_URL: string | undefined;
}

import type { ClientConfigInput } from "@/client/client";
import { VERSION } from "@/utils";

const DEVTOOLS_URL = (version = VERSION) =>
	`https://releases.rivet.dev/rivet/latest/devtools/mod.js?v=${version}`;

const scriptId = "rivetkit-devtools-script";

export function injectDevtools(config: ClientConfigInput) {
	if (!document.getElementById(scriptId)) {
		const script = document.createElement("script");
		script.id = scriptId;
		script.src = globalThis.CUSTOM_RIVETKIT_DEVTOOLS_URL || DEVTOOLS_URL();
		script.async = true;
		document.head.appendChild(script);
	}

	window.__rivetkit = window.__rivetkit || [];
	window.__rivetkit.push(config);
	return;
}
