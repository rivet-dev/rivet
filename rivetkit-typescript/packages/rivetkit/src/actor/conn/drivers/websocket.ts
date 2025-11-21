import type { WSContext } from "hono/ws";
import type { AnyConn } from "@/actor/conn/mod";
import type { AnyActorInstance } from "@/actor/instance/mod";
import type { CachedSerializer, Encoding } from "@/actor/protocol/serde";
import { loggerWithoutContext } from "../../log";
import { type ConnDriver, DriverReadyState } from "../driver";

export type ConnDriverWebSocketState = Record<never, never>;

export function createWebSocketDriver(
	hibernatable: ConnDriver['hibernatable'],
	encoding: Encoding,
	closePromise: Promise<void>,
): { driver: ConnDriver; setWebSocket(ws: WSContext): void } {
	loggerWithoutContext().debug({
		msg: "createWebSocketDriver creating driver",
		hibernatable,
	});
	// Wait for WS to open
	let websocket: WSContext | undefined;

	const driver: ConnDriver = {
		type: "websocket",
		hibernatable,
		rivetKitProtocol: {
			sendMessage: (
				actor: AnyActorInstance,
				conn: AnyConn,
				message: CachedSerializer<any, any, any>,
			) => {
				if (!websocket) {
					actor.rLog.warn({
						msg: "websocket not open",
						connId: conn.id,
					});
					return;
				}
				if (websocket.readyState !== DriverReadyState.OPEN) {
					actor.rLog.warn({
						msg: "attempting to send message to closed websocket, this is likely a bug in RivetKit",
						connId: conn.id,
						wsReadyState: websocket.readyState,
					});
					return;
				}

				const serialized = message.serialize(encoding);

				actor.rLog.debug({
					msg: "sending websocket message",
					encoding: encoding,
					dataType: typeof serialized,
					isUint8Array: serialized instanceof Uint8Array,
					isArrayBuffer: serialized instanceof ArrayBuffer,
					dataLength:
						(serialized as any).byteLength ||
						(serialized as any).length,
				});

				// Convert Uint8Array to ArrayBuffer for proper transmission
				if (serialized instanceof Uint8Array) {
					const buffer = serialized.buffer.slice(
						serialized.byteOffset,
						serialized.byteOffset + serialized.byteLength,
					);
					// Handle SharedArrayBuffer case
					if (buffer instanceof SharedArrayBuffer) {
						const arrayBuffer = new ArrayBuffer(buffer.byteLength);
						new Uint8Array(arrayBuffer).set(new Uint8Array(buffer));
						actor.rLog.debug({
							msg: "converted SharedArrayBuffer to ArrayBuffer",
							byteLength: arrayBuffer.byteLength,
						});
						websocket.send(arrayBuffer);
					} else {
						actor.rLog.debug({
							msg: "sending ArrayBuffer",
							byteLength: buffer.byteLength,
						});
						websocket.send(buffer);
					}
				} else {
					actor.rLog.debug({
						msg: "sending string data",
						length: (serialized as string).length,
					});
					websocket.send(serialized);
				}
			},
		},

		disconnect: async (
			_actor: AnyActorInstance,
			_conn: AnyConn,
			reason?: string,
		) => {
			if (!websocket) {
				loggerWithoutContext().warn(
					"disconnecting ws without websocket",
				);
				return;
			}

			// Close socket
			websocket.close(1000, reason);

			// Create promise to wait for socket to close gracefully
			await closePromise;
		},

		terminate: () => {
			(websocket as any).terminate();
		},

		getConnectionReadyState: (
			_actor: AnyActorInstance,
			_conn: AnyConn,
		): DriverReadyState | undefined => {
			return websocket?.readyState ?? DriverReadyState.CONNECTING;
		},
	};

	return {
		driver,
		setWebSocket(ws) {
			websocket = ws;
		},
	};
}
