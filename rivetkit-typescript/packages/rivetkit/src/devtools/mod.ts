/// <reference lib="dom" />

declare global {
	interface Window {
		_rivetkit_devtools_configs?: ClientConfigInput[];
	}
	// injected via tsup config
	var CUSTOM_RIVETKIT_DEVTOOLS_URL: string | undefined;
}

import type { ClientConfigInput } from "@/client/client";
import { VERSION } from "@/utils";

const DEVTOOLS_URL = (version = VERSION) =>
	`https://releases.rivet.gg/devtools/${version}/rivetkit-devtools.js`;

const scriptId = "rivetkit-devtools-script";

export function injectDevtools(config: ClientConfigInput) {
	if (!document.getElementById(scriptId)) {
		const script = document.createElement("script");
		script.id = scriptId;
		script.src = globalThis.CUSTOM_RIVETKIT_DEVTOOLS_URL || DEVTOOLS_URL();
		script.async = true;
		document.head.appendChild(script);
	}

	window._rivetkit_devtools_configs = window._rivetkit_devtools_configs || [];
	window._rivetkit_devtools_configs.push(config);
	return;
}
