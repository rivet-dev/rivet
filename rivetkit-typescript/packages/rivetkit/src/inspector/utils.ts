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
	httpPort: number,
): string | undefined {
	if (!config.inspector.enabled) return undefined;

	// Prefer the engine endpoint for the inspector URL when we know the engine
	// serves the UI locally. The engine always hosts `/ui/` on its own port
	// (6420 by default) so pointing users at the engine URL keeps the
	// inspector discoverable at the standard Rivet port even when the
	// local RivetKit HTTP server runs on a different port (e.g. 8080).
	const base =
		config.inspector.defaultEndpoint ??
		config.endpoint ??
		`http://127.0.0.1:${httpPort}`;
	return new URL("/ui/", base).href;
}
