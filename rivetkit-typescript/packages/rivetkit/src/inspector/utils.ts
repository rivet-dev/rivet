import { createMiddleware } from "hono/factory";
import type { RegistryConfig } from "@/registry/config";
import { timingSafeEqual } from "@/utils/crypto";

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

		if (!timingSafeEqual(userToken, inspectorToken)) {
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
