import type { Registry } from "rivetkit";
import { logger } from "./log.ts";
import {
	configureRunnerVersion,
	type ConvexHandlerOptions,
	type SerializedRequest,
	type SerializedResponse,
} from "./shared.ts";

/**
 * Create a handler for Convex HTTP actions.
 *
 * This adapter allows Convex to act as a serverless runner for Rivet Cloud.
 * Rivet Cloud manages actor state and lifecycle, while Convex runs the actor
 * logic on each request.
 *
 * @example
 * ```ts
 * import { httpRouter } from "convex/server";
 * import { httpAction } from "./_generated/server";
 * import { toConvexHandler } from "@rivetkit/convex";
 * import { registry } from "./actors";
 *
 * const http = httpRouter();
 * const rivetHandler = toConvexHandler(registry);
 *
 * const methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"] as const;
 * for (const method of methods) {
 *   http.route({
 *     pathPrefix: "/api/rivet/",
 *     method,
 *     handler: httpAction(async (ctx, request) => rivetHandler(ctx, request)),
 *   });
 * }
 *
 * export default http;
 * ```
 */
export function toConvexHandler<A extends Registry<any>>(
	registry: A,
	options?: ConvexHandlerOptions,
): (ctx: any, request: Request) => Promise<Response> {
	logger().debug("initializing convex handler");

	// Don't run server locally since we're using the fetch handler directly.
	registry.config.serveManager = false;

	// Set basePath since Convex route strips the prefix.
	registry.config.serverless = {
		...registry.config.serverless,
		basePath: options?.basePath ?? "/",
	};

	// Convex logs handler invocations, so no need for a welcome message.
	registry.config.noWelcome = true;

	// Enable hot-reload in development
	configureRunnerVersion(registry);

	return async (_ctx: any, request: Request): Promise<Response> => {
		logger().debug({ msg: "handling request", url: request.url });
		return await registry.handler(request);
	};
}

/**
 * Serialize a Request for passing to a Convex Node.js action.
 *
 * @example
 * ```ts
 * // In http.ts
 * const serialized = await serializeRequest(request);
 * const result = await ctx.runAction(api.nodeActions.handleRivetRequest, serialized);
 * ```
 */
export async function serializeRequest(request: Request): Promise<SerializedRequest> {
	const headers: Record<string, string> = {};
	request.headers.forEach((value, key) => {
		headers[key] = value;
	});

	let body: string | undefined;
	if (request.method !== "GET" && request.method !== "OPTIONS") {
		try {
			body = await request.text();
		} catch {
			// No body
		}
	}

	return {
		method: request.method,
		url: request.url,
		headers,
		body,
	};
}

/**
 * Deserialize a response from a Convex Node.js action back to a Response.
 *
 * @example
 * ```ts
 * // In http.ts
 * const result = await ctx.runAction(api.nodeActions.handleRivetRequest, serialized);
 * return deserializeResponse(result);
 * ```
 */
export function deserializeResponse(result: SerializedResponse): Response {
	return new Response(result.body, {
		status: result.status,
		statusText: result.statusText,
		headers: result.headers,
	});
}

const METHODS = ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"] as const;

/**
 * Add RivetKit routes to a Convex HTTP router.
 *
 * This function registers all necessary HTTP method handlers that proxy requests
 * through a Node.js action for WebSocket support.
 *
 * @example
 * ```ts
 * import { httpRouter } from "convex/server";
 * import { httpAction } from "./_generated/server";
 * import { api } from "./_generated/api";
 * import { addRivetRoutes } from "@rivetkit/convex";
 *
 * const http = httpRouter();
 * addRivetRoutes(http, httpAction, api.nodeActions.handleRivetRequest);
 * export default http;
 * ```
 */
export function addRivetRoutes(
	router: any,
	httpAction: any,
	actionRef: any,
	options?: { pathPrefix?: string },
) {
	const pathPrefix = options?.pathPrefix ?? "/api/rivet/";

	for (const method of METHODS) {
		router.route({
			pathPrefix,
			method,
			handler: httpAction(async (ctx: any, request: Request) => {
				const serialized = await serializeRequest(request);
				const result = await ctx.runAction(actionRef, serialized);
				return deserializeResponse(result);
			}),
		});
	}
}
