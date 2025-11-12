import type { RegistryConfig } from "@/mod";
import { getNodeCrypto } from "@/utils/node";

export function compareSecrets(providedSecret: string, validSecret: string) {
	// Early length check to avoid unnecessary processing
	if (providedSecret.length !== validSecret.length) {
		return false;
	}

	const encoder = new TextEncoder();

	const a = encoder.encode(providedSecret);
	const b = encoder.encode(validSecret);

	if (a.byteLength !== b.byteLength) {
		return false;
	}

	// Perform timing-safe comparison
	if (!getNodeCrypto().timingSafeEqual(a, b)) {
		return false;
	}
	return true;
}

export function getInspectorUrl(runConfig: RegistryConfig | undefined) {
	const url = new URL("https://inspect.rivet.dev");

	const overrideDefaultEndpoint = runConfig?.inspector?.defaultEndpoint;
	if (overrideDefaultEndpoint) {
		url.searchParams.set("u", overrideDefaultEndpoint);
	}

	return url.href;
}
