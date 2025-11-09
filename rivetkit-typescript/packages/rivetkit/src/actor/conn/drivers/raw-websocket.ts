import type { AnyConn } from "@/actor/conn/mod";
import type { AnyActorInstance } from "@/actor/instance/mod";
import type { UniversalWebSocket } from "@/common/websocket-interface";
import type { ConnDriver, DriverReadyState } from "../driver";

/**
 * Creates a raw WebSocket connection driver.
 *
 * This driver is used for raw WebSocket connections that don't use the RivetKit protocol.
 * Unlike the standard WebSocket driver, this doesn't have sendMessage since raw WebSockets
 * don't handle messages from the RivetKit protocol - they handle messages directly in the
 * actor's onWebSocket handler.
 */
export function createRawWebSocketSocket(
	requestId: string,
	requestIdBuf: ArrayBuffer | undefined,
	hibernatable: boolean,
	websocket: UniversalWebSocket,
	closePromise: Promise<void>,
): ConnDriver {
	return {
		requestId,
		requestIdBuf,
		hibernatable,

		// No sendMessage implementation since this is a raw WebSocket that doesn't
		// handle messages from the RivetKit protocol

		disconnect: async (
			_actor: AnyActorInstance,
			_conn: AnyConn,
			reason?: string,
		) => {
			// Close socket
			websocket.close(1000, reason);

			// Wait for socket to close gracefully
			await closePromise;
		},

		terminate: () => {
			(websocket as any).terminate?.();
		},

		getConnectionReadyState: (
			_actor: AnyActorInstance,
			_conn: AnyConn,
		): DriverReadyState | undefined => {
			return websocket.readyState;
		},
	};
}
