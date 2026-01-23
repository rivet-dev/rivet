import type { Registry } from "rivetkit";
import { v } from "convex/values";
import { logger } from "./log.ts";
import {
	configureRunnerVersion,
	type ConvexHandlerOptions,
	type SerializedRequest,
	type SerializedResponse,
} from "./shared.ts";

/**
 * Create a handler for Convex Node.js actions that processes RivetKit requests.
 *
 * This is used when you need WebSocket support, which is only available in
 * Convex Node.js actions (not HTTP actions).
 *
 * @example
 * ```ts
 * "use node";
 *
 * import WS from "ws";
 * import { injectWebSocketSync, createNodeActionHandler } from "@rivetkit/convex";
 * injectWebSocketSync(WS);
 *
 * import { action } from "./_generated/server";
 * import { v } from "convex/values";
 * import { registry } from "./actors";
 *
 * const handleRequest = createNodeActionHandler(registry, { basePath: "/api/rivet" });
 *
 * export const handleRivetRequest = action({
 *   args: {
 *     method: v.string(),
 *     url: v.string(),
 *     headers: v.any(),
 *     body: v.optional(v.string()),
 *   },
 *   handler: async (_ctx, args) => handleRequest(args),
 * });
 * ```
 */
export function createNodeActionHandler<A extends Registry<any>>(
	registry: A,
	options?: ConvexHandlerOptions,
): (args: SerializedRequest) => Promise<SerializedResponse> {
	// Configure for serverless
	registry.config.serveManager = false;
	registry.config.serverless = {
		...registry.config.serverless,
		basePath: options?.basePath ?? "/",
	};
	registry.config.noWelcome = true;

	// Enable hot-reload in development
	configureRunnerVersion(registry);

	return async (args: SerializedRequest): Promise<SerializedResponse> => {
		logger().debug({ msg: "handling node action request", method: args.method, url: args.url });

		// Build request
		const request = new Request(args.url, {
			method: args.method,
			headers: new Headers(args.headers),
			body: args.body,
		});

		// Handle request
		const response = await registry.handler(request);

		// For SSE endpoints like /start, keep the connection alive
		const url = new URL(args.url);
		if (url.pathname.endsWith("/start")) {
			// Keep the action running to maintain WebSocket connection
			// The SSE stream never completes, so we wait indefinitely
			await new Promise(() => {});
		}

		// Convert response to serializable format
		const responseBody = await response.text();
		const responseHeaders: Record<string, string> = {};
		response.headers.forEach((value, key) => {
			responseHeaders[key] = value;
		});

		return {
			status: response.status,
			statusText: response.statusText,
			headers: responseHeaders,
			body: responseBody,
		};
	};
}

/**
 * Create an action definition for Convex Node.js actions that processes RivetKit requests.
 *
 * This is the simplest way to integrate RivetKit with Convex. The returned object can be
 * spread directly into Convex's `action()` function.
 *
 * @example
 * ```ts
 * "use node";
 *
 * import { action } from "./_generated/server";
 * import { createRivetAction } from "@rivetkit/convex";
 * import { registry } from "./actors";
 *
 * export const handleRivetRequest = action(createRivetAction(registry));
 * ```
 */
export function createRivetAction<A extends Registry<any>>(
	registry: A,
	options?: ConvexHandlerOptions,
) {
	const basePath = options?.basePath ?? "/api/rivet";

	// Configure registry for serverless
	registry.config.serveManager = false;
	registry.config.serverless = {
		...registry.config.serverless,
		basePath,
	};
	registry.config.noWelcome = true;

	// Enable hot-reload in development
	configureRunnerVersion(registry);

	return {
		args: {
			method: v.string(),
			url: v.string(),
			headers: v.any(),
			body: v.optional(v.string()),
		},
		handler: async (_ctx: any, args: SerializedRequest): Promise<SerializedResponse> => {
			logger().debug({ msg: "handling rivet action request", method: args.method, url: args.url });

			const request = new Request(args.url, {
				method: args.method,
				headers: new Headers(args.headers),
				body: args.body,
			});
			const response = await registry.handler(request);

			// For SSE endpoints like /start, keep the connection alive
			const url = new URL(args.url);
			if (url.pathname.endsWith("/start")) {
				await new Promise(() => {});
			}

			const responseBody = await response.text();
			const responseHeaders: Record<string, string> = {};
			response.headers.forEach((value, key) => {
				responseHeaders[key] = value;
			});

			return {
				status: response.status,
				statusText: response.statusText,
				headers: responseHeaders,
				body: responseBody,
			};
		},
	};
}
