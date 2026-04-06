// Lazy PostHog wrapper. Keeps posthog-js out of the main bundle.
// initPosthog() is called from initThirdPartyProviders() after app startup.
// Calls made before the SDK loads are queued and flushed once it resolves.

import type { default as PostHogType } from "posthog-js";

type CaptureArgs = Parameters<typeof PostHogType["capture"]>;
type SetPersonPropertiesArgs = Parameters<typeof PostHogType["setPersonProperties"]>;
type QueuedCall =
	| { method: "capture"; args: CaptureArgs }
	| { method: "setPersonProperties"; args: SetPersonPropertiesArgs };

let queue: QueuedCall[] = [];
let instance: typeof PostHogType | null = null;

export async function initPosthog(apiKey: string, apiHost: string, debug: boolean) {
	const { default: ph } = await import("posthog-js");
	ph.init(apiKey, { api_host: apiHost, debug });
	instance = ph;
	for (const call of queue) {
		(ph[call.method] as (...args: unknown[]) => void)(...call.args);
	}
	queue = [];
	return ph;
}

function capture(...args: CaptureArgs) {
	if (instance) {
		instance.capture(...args);
	} else {
		queue.push({ method: "capture", args });
	}
}

function setPersonProperties(...args: SetPersonPropertiesArgs) {
	if (instance) {
		instance.setPersonProperties(...args);
	} else {
		queue.push({ method: "setPersonProperties", args });
	}
}

export const posthog = { capture, setPersonProperties };
