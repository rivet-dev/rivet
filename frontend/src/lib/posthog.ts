// Lazy PostHog wrapper. Keeps posthog-js out of the main bundle.
// initPosthog() is called from initThirdPartyProviders() after app startup.
// Calls made before the SDK loads are queued and flushed once it resolves.

import type { default as PostHogType } from "posthog-js";

type CaptureArgs = Parameters<(typeof PostHogType)["capture"]>;
type SetPersonPropertiesArgs = Parameters<
	(typeof PostHogType)["setPersonProperties"]
>;
type QueuedCall =
	| { method: "capture"; args: CaptureArgs }
	| { method: "setPersonProperties"; args: SetPersonPropertiesArgs };

const FEATURE_FLAG_SNAPSHOT_TIMEOUT_MS = 1500;

let queue: QueuedCall[] = [];
let instance: typeof PostHogType | null = null;
let enabledFeatureFlags: ReadonlySet<string> = new Set();

export async function initPosthog(
	apiKey: string,
	apiHost: string,
	debug: boolean,
	overrides?: Partial<Parameters<(typeof PostHogType)["init"]>[1]>,
) {
	if (instance) {
		return instance;
	}
	const { default: ph } = await import("posthog-js");
	ph.init(apiKey, { api_host: apiHost, debug, ...overrides });
	instance = ph;
	for (const call of queue) {
		(ph[call.method] as (...args: unknown[]) => void)(...call.args);
	}
	queue = [];
	return ph;
}

/**
 * Loads PostHog and records the enabled feature flags exactly once, resolving
 * on the first `onFeatureFlags` emission or when the timeout elapses.
 *
 * Later emissions are ignored: `features` is read at module-evaluation time by
 * several modules and guards hook order in others, so the flag set must not
 * change once the app has started.
 */
export async function snapshotPosthogFeatureFlags(
	apiKey: string,
	apiHost: string,
	debug: boolean,
) {
	const ph = await initPosthog(apiKey, apiHost, debug);

	let settled = false;
	await new Promise<void>((resolve) => {
		const timeout = setTimeout(() => {
			settled = true;
			resolve();
		}, FEATURE_FLAG_SNAPSHOT_TIMEOUT_MS);

		ph.onFeatureFlags((flags) => {
			if (settled) {
				return;
			}
			settled = true;
			enabledFeatureFlags = new Set(flags);
			clearTimeout(timeout);
			resolve();
		});
	});
}

export function getPosthogEnabledFeatureFlags(): ReadonlySet<string> {
	return enabledFeatureFlags;
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
