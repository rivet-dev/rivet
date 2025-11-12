// import crypto from "node:crypto";
import { createMiddleware } from "hono/factory";
import type { RunConfig } from "@/mod";
import type { RunnerConfigInput } from "@/registry/run-config";
import { inspectorLogger } from "./log";

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

	// TODO:
	// // Perform timing-safe comparison
	// if (!crypto.timingSafeEqual(a, b)) {
	// 	return false;
	// }
	return true;
}

export function getInspectorUrl(runConfig: RunnerConfigInput | undefined) {
	const url = new URL("https://inspect.rivet.dev");

	const overrideDefaultEndpoint =
		runConfig?.inspector?.defaultEndpoint ??
		runConfig?.overrideServerAddress;
	if (overrideDefaultEndpoint) {
		url.searchParams.set("u", overrideDefaultEndpoint);
	}

	return url.href;
}
