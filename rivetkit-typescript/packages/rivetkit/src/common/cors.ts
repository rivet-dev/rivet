import type { MiddlewareHandler } from "hono";

/**
 * Simple CORS middleware that matches the gateway behavior.
 *
 * - Echoes back the Origin header from the request
 * - Echoes back the Access-Control-Request-Headers from preflight
 * - Supports credentials
 * - Allows common HTTP methods
 * - Caches preflight for 24 hours
 * - Adds Vary header to prevent cache poisoning
 */
export const cors = (): MiddlewareHandler => {
	return async (c, next) => {
		// Extract origin from request
		const origin = c.req.header("origin") || "*";

		// Handle preflight OPTIONS request
		if (c.req.method === "OPTIONS") {
			const requestHeaders =
				c.req.header("access-control-request-headers") || "*";

			c.header("access-control-allow-origin", origin);
			c.header("access-control-allow-credentials", "true");
			c.header(
				"access-control-allow-methods",
				"GET, POST, PUT, DELETE, OPTIONS, PATCH",
			);
			c.header("access-control-allow-headers", requestHeaders);
			c.header("access-control-expose-headers", "*");
			c.header("access-control-max-age", "86400");

			// Add Vary header to prevent cache poisoning when echoing origin
			if (origin !== "*") {
				c.header("vary", "Origin");
			}

			// Remove content headers from preflight response
			c.res.headers.delete("content-length");
			c.res.headers.delete("content-type");

			return c.body(null, 204);
		}

		await next();

		// Add CORS headers to actual request
		c.header("access-control-allow-origin", origin);
		c.header("access-control-allow-credentials", "true");
		c.header("access-control-expose-headers", "*");

		// Add Vary header to prevent cache poisoning when echoing origin
		if (origin !== "*") {
			c.header("vary", "Origin");
		}
	};
};
