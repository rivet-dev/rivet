import type { ClientConfigInput } from "@/client/client";
import { logger } from "./log";

declare global {
	// Injected via tsup config
	var CUSTOM_RIVETKIT_DEVTOOLS_URL: string | undefined;
}

const scriptId = "rivetkit-devtools-script";

export function injectDevtools(config: ClientConfigInput) {
	if (!window) {
		logger().warn("devtools not available outside browser environment");
		return;
	}

	if (!document.getElementById(scriptId)) {
		const src =
			globalThis.CUSTOM_RIVETKIT_DEVTOOLS_URL ||
			`${config.endpoint?.replace(/\/$/, "")}/devtools/mod.js`;
		const script = document.createElement("script");
		script.id = scriptId;
		script.src = src;
		script.async = true;
		document.head.appendChild(script);
	}

	window.__rivetkit = window.__rivetkit || [];
	window.__rivetkit.push(config);
}
