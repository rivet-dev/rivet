import { Hono } from "hono";
import invariant from "invariant";
import { EncodingSchema } from "@/actor/protocol/serde";
import {
	type ActionOpts,
	type ActionOutput,
	type ConnectWebSocketOpts,
	type ConnectWebSocketOutput,
	type ConnsMessageOpts,
	handleAction,
	handleRawWebSocketHandler,
	handleWebSocketConnect,
} from "@/actor/router-endpoints";
import {
	PATH_CONNECT,
	PATH_WEBSOCKET_PREFIX,
	WS_PROTOCOL_CONN_PARAMS,
	WS_PROTOCOL_ENCODING,
} from "@/common/actor-router-consts";
import {
	handleRouteError,
	handleRouteNotFound,
	loggerMiddleware,
} from "@/common/router";
import { noopNext } from "@/common/utils";
import {
	type ActorInspectorRouterEnv,
	createActorInspectorRouter,
} from "@/inspector/actor";
import { isInspectorEnabled, secureInspector } from "@/inspector/utils";
import type { RunnerConfig } from "@/registry/run-config";
import { CONN_DRIVER_SYMBOL, generateConnRequestId } from "./conn/mod";
import type { ActorDriver } from "./driver";
import { InternalError } from "./errors";
import { loggerWithoutContext } from "./log";

export type {
	ConnectWebSocketOpts,
	ConnectWebSocketOutput,
	ActionOpts,
	ActionOutput,
	ConnsMessageOpts,
};

interface ActorRouterBindings {
	actorId: string;
}

export type ActorRouter = Hono<{ Bindings: ActorRouterBindings }>;

/**
 * Creates a router that runs on the partitioned instance.
 */
export function createActorRouter(
	runConfig: RunnerConfig,
	actorDriver: ActorDriver,
	isTest: boolean,
): ActorRouter {
	const router = new Hono<{ Bindings: ActorRouterBindings }>({
		strict: false,
	});

	router.use("*", loggerMiddleware(loggerWithoutContext()));

	// Track all HTTP requests to prevent actor from sleeping during active requests
	router.use("*", async (c, next) => {
		const actor = await actorDriver.loadActor(c.env.actorId);
		actor.beginHonoHttpRequest();
		try {
			await next();
		} finally {
			actor.endHonoHttpRequest();
		}
	});

	router.get("/", (c) => {
		return c.text(
			"This is an RivetKit actor.\n\nLearn more at https://rivetkit.org",
		);
	});

	router.get("/health", (c) => {
		return c.text("ok");
	});

	if (isTest) {
		// Test endpoint to force disconnect a connection non-cleanly
		router.post("/.test/force-disconnect", async (c) => {
			const connId = c.req.query("conn");

			if (!connId) {
				return c.text("Missing conn query parameter", 400);
			}

			const actor = await actorDriver.loadActor(c.env.actorId);
			const conn = actor.getConnForId(connId);

			if (!conn) {
				return c.text(`Connection not found: ${connId}`, 404);
			}

			// Force close the connection without clean shutdown
			if (conn[CONN_DRIVER_SYMBOL]?.terminate) {
				conn[CONN_DRIVER_SYMBOL].terminate(actor, conn);
			}

			return c.json({ success: true });
		});
	}

	router.get(PATH_CONNECT, async (c) => {
		const upgradeWebSocket = runConfig.getUpgradeWebSocket?.();
		if (upgradeWebSocket) {
			return upgradeWebSocket(async (c) => {
				// Parse configuration from Sec-WebSocket-Protocol header
				const protocols = c.req.header("sec-websocket-protocol");
				let encodingRaw: string | undefined;
				let connParamsRaw: string | undefined;

				if (protocols) {
					const protocolList = protocols
						.split(",")
						.map((p) => p.trim());
					for (const protocol of protocolList) {
						if (protocol.startsWith(WS_PROTOCOL_ENCODING)) {
							encodingRaw = protocol.substring(
								WS_PROTOCOL_ENCODING.length,
							);
						} else if (
							protocol.startsWith(WS_PROTOCOL_CONN_PARAMS)
						) {
							connParamsRaw = decodeURIComponent(
								protocol.substring(
									WS_PROTOCOL_CONN_PARAMS.length,
								),
							);
						}
					}
				}

				const encoding = EncodingSchema.parse(encodingRaw);
				const connParams = connParamsRaw
					? JSON.parse(connParamsRaw)
					: undefined;

				return await handleWebSocketConnect(
					c.req.raw,
					runConfig,
					actorDriver,
					c.env.actorId,
					encoding,
					connParams,
					generateConnRequestId(),
					undefined,
				);
			})(c, noopNext());
		} else {
			return c.text("WebSockets are not enabled for this driver.", 400);
		}
	});

	router.post("/action/:action", async (c) => {
		const actionName = c.req.param("action");

		return handleAction(
			c,
			runConfig,
			actorDriver,
			actionName,
			c.env.actorId,
		);
	});

	// Raw HTTP endpoints - /request/*
	router.all("/request/*", async (c) => {
		const actor = await actorDriver.loadActor(c.env.actorId);

		// TODO: This is not a clean way of doing this since `/http/` might exist mid-path
		// Strip the /http prefix from the URL to get the original path
		const url = new URL(c.req.url);
		const originalPath = url.pathname.replace(/^\/raw\/http/, "") || "/";

		// Create a new request with the corrected URL
		const correctedUrl = new URL(originalPath + url.search, url.origin);
		const correctedRequest = new Request(correctedUrl, {
			method: c.req.method,
			headers: c.req.raw.headers,
			body: c.req.raw.body,
			duplex: "half",
		} as RequestInit);

		loggerWithoutContext().debug({
			msg: "rewriting http url",
			from: c.req.url,
			to: correctedRequest.url,
		});

		// Call the actor's onRequest handler - it will throw appropriate errors
		const response = await actor.handleRawRequest(correctedRequest, {});

		// This should never happen now since handleFetch throws errors
		if (!response) {
			throw new InternalError("handleFetch returned void unexpectedly");
		}

		return response;
	});

	// Raw WebSocket endpoint - /websocket/*
	router.get(`${PATH_WEBSOCKET_PREFIX}*`, async (c) => {
		const upgradeWebSocket = runConfig.getUpgradeWebSocket?.();
		if (upgradeWebSocket) {
			return upgradeWebSocket(async (c) => {
				const url = new URL(c.req.url);
				const pathWithQuery = c.req.path + url.search;

				loggerWithoutContext().debug({
					msg: "actor router raw websocket",
					path: c.req.path,
					url: c.req.url,
					search: url.search,
					pathWithQuery,
				});

				return await handleRawWebSocketHandler(
					c.req.raw,
					pathWithQuery,
					actorDriver,
					c.env.actorId,
					undefined,
				);
			})(c, noopNext());
		} else {
			return c.text("WebSockets are not enabled for this driver.", 400);
		}
	});

	if (isInspectorEnabled(runConfig, "actor")) {
		router.route(
			"/inspect",
			new Hono<
				ActorInspectorRouterEnv & { Bindings: ActorRouterBindings }
			>()
				.use(secureInspector(runConfig), async (c, next) => {
					const inspector = (
						await actorDriver.loadActor(c.env.actorId)
					).inspector;
					invariant(
						inspector,
						"inspector not supported on this platform",
					);

					c.set("inspector", inspector);
					return next();
				})
				.route("/", createActorInspectorRouter()),
		);
	}

	router.notFound(handleRouteNotFound);
	router.onError(handleRouteError);

	return router;
}
