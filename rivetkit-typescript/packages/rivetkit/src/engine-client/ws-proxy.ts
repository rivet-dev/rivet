import type { Context as HonoContext } from "hono";
import type { WSContext } from "hono/ws";
import type { UpgradeWebSocketArgs } from "@/actor/router-websocket-endpoints";
import { stringifyError } from "@/common/utils";
import { importWebSocket } from "@/common/websocket";
import { logger } from "./log";

/**
 * Returns Hono `upgradeWebSocket` args that will proxy requests from the client to a destination address.
 */
export async function createWebSocketProxy(
	c: HonoContext,
	targetUrl: string,
	protocols: string[],
): Promise<UpgradeWebSocketArgs> {
	const WebSocket = await importWebSocket();

	// WebSocket state
	interface WsState {
		targetWs?: WebSocket;
		connectPromise?: Promise<void>;
	}
	const state: WsState = {};

	return {
		onOpen: async (event: any, clientWs: WSContext) => {
			logger().debug({ msg: "client websocket connected", targetUrl });

			if (clientWs.readyState !== 1) {
				logger().warn({
					msg: "client websocket not open on connection",
					targetUrl,
					readyState: clientWs.readyState,
				});
				return;
			}

			// Create WebSocket
			const targetWs = new WebSocket(targetUrl, protocols);
			state.targetWs = targetWs;

			// Setup connection promise
			state.connectPromise = new Promise<void>((resolve, reject) => {
				targetWs.addEventListener("open", () => {
					logger().debug({
						msg: "target websocket connected",
						targetUrl,
					});

					if (clientWs.readyState !== 1) {
						logger().warn({
							msg: "client websocket closed before target connected",
							targetUrl,
							clientReadyState: clientWs.readyState,
						});
						targetWs.close(1001, "Client disconnected");
						reject(new Error("Client disconnected"));
						return;
					}
					resolve();
				});

				targetWs.addEventListener("error", (error) => {
					logger().warn({
						msg: "target websocket error during connection",
						targetUrl,
					});
					reject(error);
				});
			});

			// Setup bidirectional forwarding
			state.targetWs.addEventListener("message", (event) => {
				if (
					typeof event.data === "string" ||
					event.data instanceof ArrayBuffer
				) {
					clientWs.send(event.data);
				} else if (event.data instanceof Blob) {
					event.data.arrayBuffer().then((buffer) => {
						clientWs.send(buffer);
					});
				}
			});

			state.targetWs.addEventListener("close", (event) => {
				logger().debug({
					msg: "target websocket closed",
					targetUrl,
					code: event.code,
					reason: event.reason,
				});
				closeWebSocketIfOpen(clientWs, event.code, event.reason);
			});

			state.targetWs.addEventListener("error", (error) => {
				logger().error({
					msg: "target websocket error",
					targetUrl,
					error: stringifyError(error),
				});
				closeWebSocketIfOpen(clientWs, 1011, "Target WebSocket error");
			});
		},

		onMessage: async (event: any, clientWs: WSContext) => {
			if (!state.targetWs || !state.connectPromise) {
				logger().error({
					msg: "websocket state not initialized",
					targetUrl,
				});
				return;
			}

			try {
				await state.connectPromise;
				if (state.targetWs.readyState === WebSocket.OPEN) {
					state.targetWs.send(event.data);
				} else {
					logger().warn({
						msg: "target websocket not open",
						targetUrl,
						readyState: state.targetWs.readyState,
					});
				}
			} catch (error) {
				logger().error({
					msg: "failed to connect to target websocket",
					targetUrl,
					error,
				});
				closeWebSocketIfOpen(
					clientWs,
					1011,
					"Failed to connect to target",
				);
			}
		},

		onClose: (event: any, clientWs: WSContext) => {
			logger().debug({
				msg: "client websocket closed",
				targetUrl,
				code: event.code,
				reason: event.reason,
				wasClean: event.wasClean,
			});

			if (state.targetWs) {
				if (
					state.targetWs.readyState === WebSocket.OPEN ||
					state.targetWs.readyState === WebSocket.CONNECTING
				) {
					state.targetWs.close(
						1000,
						event.reason || "Client disconnected",
					);
				}
			}
		},

		onError: (event: any, clientWs: WSContext) => {
			logger().error({ msg: "client websocket error", targetUrl, event });

			if (state.targetWs) {
				if (state.targetWs.readyState === WebSocket.OPEN) {
					state.targetWs.close(1011, "Client WebSocket error");
				} else if (state.targetWs.readyState === WebSocket.CONNECTING) {
					state.targetWs.close();
				}
			}
		},
	};
}

function closeWebSocketIfOpen(
	ws: WebSocket | WSContext,
	code: number,
	reason: string,
): void {
	if (ws.readyState === 1) {
		ws.close(code, reason);
	} else if (
		"close" in ws &&
		(ws as WebSocket).readyState === WebSocket.OPEN
	) {
		ws.close(code, reason);
	}
}
