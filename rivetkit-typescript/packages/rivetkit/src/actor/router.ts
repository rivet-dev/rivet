import { Hono } from "hono";
import invariant from "invariant";
import {
	type ActionOpts,
	type ActionOutput,
	type ConnsMessageOpts,
	handleAction,
	handleRawRequest,
} from "@/actor/router-endpoints";
import {
	PATH_CONNECT,
	PATH_INSPECTOR_CONNECT,
	PATH_WEBSOCKET_PREFIX,
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
import { loggerWithoutContext } from "./log";
import {
	parseWebSocketProtocols,
	routeWebSocket,
} from "./router-websocket-endpoints";

export type { ActionOpts, ActionOutput, ConnsMessageOpts };

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
			const conn = actor.connectionManager.getConnForId(connId);

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

	// Route all WebSocket paths using the same handler
	//
	// All WebSockets use a separate underlying router in routeWebSocket since
	// WebSockets also need to be routed from ManagerDriver.proxyWebSocket and
	// ManagerDriver.openWebSocket.
	router.on(
		"GET",
		[PATH_CONNECT, `${PATH_WEBSOCKET_PREFIX}*`, PATH_INSPECTOR_CONNECT],
		async (c) => {
			const upgradeWebSocket = runConfig.getUpgradeWebSocket?.();
			if (upgradeWebSocket) {
				return upgradeWebSocket(async (c) => {
					const protocols = c.req.header("sec-websocket-protocol");
					const { encoding, connParams } =
						parseWebSocketProtocols(protocols);

					return await routeWebSocket(
						c.req.raw,
						c.req.path,
						c.req.header(),
						runConfig,
						actorDriver,
						c.env.actorId,
						encoding,
						connParams,
						generateConnRequestId(),
						undefined,
						false,
						false,
					);
				})(c, noopNext());
			} else {
				return c.text(
					"WebSockets are not enabled for this driver.",
					400,
				);
			}
		},
	);

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

	router.all("/request/*", async (c) => {
		// TODO: This is not a clean way of doing this since `/http/` might exist mid-path
		// Strip the /http prefix from the URL to get the original path
		const url = new URL(c.req.url);
		const originalPath = url.pathname.replace(/^\/request/, "") || "/";

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

		return await handleRawRequest(
			c,
			correctedRequest,
			actorDriver,
			c.env.actorId,
		);
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
