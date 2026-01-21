import { OpenAPIHono } from "@hono/zod-openapi";
import type { Hono } from "hono";
import { createMiddleware } from "hono/factory";
import { cors } from "@/common/cors";
import {
	handleRouteError,
	handleRouteNotFound,
	loggerMiddleware,
} from "@/common/router";
import { getLogger } from "@/common/log";

export function logger() {
	return getLogger("router");
}

export function createRouter(
	basePath: string,
	builder: (app: OpenAPIHono) => void,
): {
	router: Hono;
	openapi: OpenAPIHono;
} {
	const router = new OpenAPIHono({ strict: false }).basePath(basePath);

	router.use("*", loggerMiddleware(logger()), cors(), async (c, next) => {
		console.log(`[rivet] ${c.req.method} ${c.req.url}`);
		await next();
	});

	// HACK: Add Sec-WebSocket-Protocol header to fix KIT-339
	//
	// Some Deno WebSocket providers do not auto-set the protocol, which
	// will cause some WebSocket clients to fail
	router.use(
		"*",
		createMiddleware(async (c, next) => {
			const upgrade = c.req.header("upgrade");
			const isWebSocket = upgrade?.toLowerCase() === "websocket";
			const isGet = c.req.method === "GET";

			if (isGet && isWebSocket) {
				c.header("Sec-WebSocket-Protocol", "rivet");
			}

			await next();
		}),
	);

	builder(router);

	// Error handling
	router.notFound(handleRouteNotFound);
	router.onError(handleRouteError);

	return { router: router as Hono, openapi: router };
}

export function buildOpenApiResponses<T>(schema: T) {
	return {
		200: {
			description: "Success",
			content: {
				"application/json": {
					schema,
				},
			},
		},
		400: {
			description: "User error",
		},
		500: {
			description: "Internal error",
		},
	};
}

export function buildOpenApiRequestBody<T>(schema: T) {
	return {
		required: true,
		content: {
			"application/json": {
				schema,
			},
		},
	};
}
