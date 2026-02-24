import { Hono } from "hono";
import {
	type ActionOpts,
	type ActionOutput,
	type ConnsMessageOpts,
	handleAction,
	handleQueueSend,
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
import { inspectorLogger } from "@/inspector/log";
import type { RegistryConfig } from "@/registry/config";
import { type GetUpgradeWebSocket, VERSION } from "@/utils";
import { timingSafeEqual } from "@/utils/crypto";
import { isDev } from "@/utils/env-vars";
import { CONN_DRIVER_SYMBOL } from "./conn/mod";
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

export interface MetadataResponse {
	runtime: string;
	version: string;
}

/**
 * Creates a router that runs on the partitioned instance.
 *
 * You only need to pass `getUpgradeWebSocket` if this router is exposed
 * directly publicly. Usually WebSockets are routed manually in the
 * ManagerDriver instead of via Hono. The only platform that uses this
 * currently is Cloudflare Workers.
 */
export function createActorRouter(
	config: RegistryConfig,
	actorDriver: ActorDriver,
	getUpgradeWebSocket: GetUpgradeWebSocket | undefined,
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

	router.get("/metadata", async (c) => {
		return c.json({
			runtime: "rivetkit",
			version: VERSION,
		} satisfies MetadataResponse);
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
	if (getUpgradeWebSocket) {
		router.on(
			"GET",
			[PATH_CONNECT, `${PATH_WEBSOCKET_PREFIX}*`, PATH_INSPECTOR_CONNECT],
			async (c) => {
				const upgradeWebSocket = getUpgradeWebSocket();
				if (upgradeWebSocket) {
					return upgradeWebSocket(async (c) => {
						const protocols = c.req.header(
							"sec-websocket-protocol",
						);
						const { encoding, connParams } =
							parseWebSocketProtocols(protocols);

						return await routeWebSocket(
							c.req.raw,
							c.req.path,
							c.req.header(),
							config,
							actorDriver,
							c.env.actorId,
							encoding,
							connParams,
							undefined,
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
	}

	// Inspector HTTP endpoints for agent-based debugging
	if (config.inspector.enabled) {
		// Auth middleware for inspector routes
		const inspectorAuth = async (c: any): Promise<Response | undefined> => {
			if (isDev() && !config.inspector.token()) {
				inspectorLogger().warn({
					msg: "RIVET_INSPECTOR_TOKEN is not set, skipping inspector auth in development mode",
				});
				return undefined;
			}

			const userToken = c.req
				.header("Authorization")
				?.replace("Bearer ", "");
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

			return undefined;
		};

		router.get("/inspector/state", async (c) => {
			const authResponse = await inspectorAuth(c);
			if (authResponse) return authResponse;

			const actor = await actorDriver.loadActor(c.env.actorId);
			const isStateEnabled = actor.inspector.isStateEnabled();
			const state = isStateEnabled
				? actor.inspector.getStateJson()
				: undefined;
			return c.json({ state, isStateEnabled });
		});

		router.patch("/inspector/state", async (c) => {
			const authResponse = await inspectorAuth(c);
			if (authResponse) return authResponse;

			const actor = await actorDriver.loadActor(c.env.actorId);
			const body = await c.req.json<{ state: unknown }>();
			await actor.inspector.setStateJson(body.state);
			return c.json({ ok: true });
		});

		router.get("/inspector/connections", async (c) => {
			const authResponse = await inspectorAuth(c);
			if (authResponse) return authResponse;

			const actor = await actorDriver.loadActor(c.env.actorId);
			const connections = actor.inspector.getConnectionsJson();
			return c.json({ connections });
		});

		router.get("/inspector/rpcs", async (c) => {
			const authResponse = await inspectorAuth(c);
			if (authResponse) return authResponse;

			const actor = await actorDriver.loadActor(c.env.actorId);
			const rpcs = actor.inspector.getRpcs();
			return c.json({ rpcs });
		});

		router.post("/inspector/action/:name", async (c) => {
			const authResponse = await inspectorAuth(c);
			if (authResponse) return authResponse;

			const actor = await actorDriver.loadActor(c.env.actorId);
			const name = c.req.param("name");
			const body = await c.req.json<{ args: unknown[] }>();
			const output = await actor.inspector.executeActionJson(
				name,
				body.args ?? [],
			);
			return c.json({ output });
		});

		router.get("/inspector/queue", async (c) => {
			const authResponse = await inspectorAuth(c);
			if (authResponse) return authResponse;

			const actor = await actorDriver.loadActor(c.env.actorId);
			const limit = parseInt(c.req.query("limit") ?? "50", 10);
			const status = await actor.inspector.getQueueStatusJson(limit);
			return c.json(status);
		});

		router.get("/inspector/traces", async (c) => {
			const authResponse = await inspectorAuth(c);
			if (authResponse) return authResponse;

			const actor = await actorDriver.loadActor(c.env.actorId);
			const startMs = parseInt(c.req.query("startMs") ?? "0", 10);
			const endMs = parseInt(
				c.req.query("endMs") ?? String(Date.now()),
				10,
			);
			const limit = parseInt(c.req.query("limit") ?? "1000", 10);

			await actor.traces.flush();
			const result = await actor.inspector.getTracesJson({
				startMs,
				endMs,
				limit,
			});
			return c.json(result);
		});

		router.get("/inspector/workflow-history", async (c) => {
			const authResponse = await inspectorAuth(c);
			if (authResponse) return authResponse;

			const actor = await actorDriver.loadActor(c.env.actorId);
			const result = actor.inspector.getWorkflowHistoryJson();
			return c.json(result);
		});

		router.get("/inspector/summary", async (c) => {
			const authResponse = await inspectorAuth(c);
			if (authResponse) return authResponse;

			const actor = await actorDriver.loadActor(c.env.actorId);

			const isStateEnabled = actor.inspector.isStateEnabled();
			const isDatabaseEnabled = actor.inspector.isDatabaseEnabled();
			const isWorkflowEnabled = actor.inspector.isWorkflowEnabled();

			const state = isStateEnabled
				? actor.inspector.getStateJson()
				: undefined;
			const connections = actor.inspector.getConnectionsJson();
			const rpcs = actor.inspector.getRpcs();
			const queueSize = actor.inspector.getQueueSize();
			const workflowHistory = actor.inspector.getWorkflowHistory();

			// Convert BigInt values in workflow history to numbers for JSON serialization.
			const bigIntReplacer = (_key: string, value: unknown) =>
				typeof value === "bigint" ? Number(value) : value;
			const safeWorkflowHistory = workflowHistory
				? JSON.parse(JSON.stringify(workflowHistory, bigIntReplacer))
				: null;

			return c.json({
				state,
				connections,
				rpcs,
				queueSize,
				isStateEnabled,
				isDatabaseEnabled,
				isWorkflowEnabled,
				workflowHistory: safeWorkflowHistory,
			});
		});
	}

	router.post("/action/:action", async (c) => {
		const actionName = c.req.param("action");

		return handleAction(c, config, actorDriver, actionName, c.env.actorId);
	});

	router.post("/queue", async (c) => {
		return handleQueueSend(c, config, actorDriver, c.env.actorId);
	});

	router.post("/queue/:name", async (c) => {
		return handleQueueSend(
			c,
			config,
			actorDriver,
			c.env.actorId,
			c.req.param("name"),
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

	router.notFound(handleRouteNotFound);
	router.onError(handleRouteError);

	return router;
}
