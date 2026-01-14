import { createMiddleware } from "hono/factory";
import { inspectorLogger } from "./log";
import type { RegistryConfig } from "@/registry/config";

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

export const secureInspector = (config: RegistryConfig) =>
	createMiddleware(async (c, next) => {
		const userToken = c.req.header("Authorization")?.replace("Bearer ", "");
		if (!userToken) {
			return c.text("Unauthorized", 401);
		}

		const inspectorToken = config.inspector.token();
		if (!inspectorToken) {
			return c.text("Unauthorized", 401);
		}

		const isValid = compareSecrets(userToken, inspectorToken);
		if (!isValid) {
			return c.text("Unauthorized", 401);
		}
		await next();
	});

export function getInspectorUrl(
	config: RegistryConfig,
	managerPort: number,
): string | undefined {
	if (!config.inspector.enabled) return undefined;

	const url = new URL("https://inspect.rivet.dev");

	// Only override endpoint if using non-default port or custom endpoint is set
	const endpoint =
		config.inspector.defaultEndpoint ??
		(config.managerPort !== 6420
			? `http://127.0.0.1:${managerPort}`
			: undefined);
	if (endpoint) {
		url.searchParams.set("u", endpoint);
	}

	return url.href;
}
