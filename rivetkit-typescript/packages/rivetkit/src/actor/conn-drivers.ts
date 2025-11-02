import type { SSEStreamingApi } from "hono/streaming";
import type { WSContext } from "hono/ws";
import type { WebSocket } from "ws";
import type { AnyConn } from "@/actor/conn";
import type { AnyActorInstance } from "@/actor/instance";
import type { CachedSerializer, Encoding } from "@/actor/protocol/serde";
import { encodeDataToString } from "@/actor/protocol/serde";
import type { HonoWebSocketAdapter } from "@/manager/hono-websocket-adapter";
import type * as protocol from "@/schemas/client-protocol/mod";
import { assertUnreachable, type promiseWithResolvers } from "@/utils";

export enum ConnDriverKind {
	WEBSOCKET = 0,
	SSE = 1,
	HTTP = 2,
}

export enum ConnReadyState {
	UNKNOWN = -1,
	CONNECTING = 0,
	OPEN = 1,
	CLOSING = 2,
	CLOSED = 3,
}

export interface ConnDriverWebSocketState {
	encoding: Encoding;
	websocket: WSContext;
	closePromise: ReturnType<typeof promiseWithResolvers<void>>;
}

export interface ConnDriverSseState {
	encoding: Encoding;
	stream: SSEStreamingApi;
}

export type ConnDriverHttpState = Record<never, never>;

export type ConnDriverState =
	| { [ConnDriverKind.WEBSOCKET]: ConnDriverWebSocketState }
	| { [ConnDriverKind.SSE]: ConnDriverSseState }
	| { [ConnDriverKind.HTTP]: ConnDriverHttpState };

export interface ConnDriver<State> {
	sendMessage?(
		actor: AnyActorInstance,
		conn: AnyConn,
		state: State,
		message: CachedSerializer<protocol.ToClient>,
	): void;

	/**
	 * This returns a promise since we commonly disconnect at the end of a program, and not waiting will cause the socket to not close cleanly.
	 */
	disconnect(
		actor: AnyActorInstance,
		conn: AnyConn,
		state: State,
		reason?: string,
	): Promise<void>;

	/**
	 * Returns the ready state of the connection.
	 * This is used to determine if the connection is ready to send messages, or if the connection is stale.
	 */
	getConnectionReadyState(
		actor: AnyActorInstance,
		conn: AnyConn,
		state: State,
	): ConnReadyState | undefined;

	/**
	 * If the underlying connection can hibernate.
	 */
	isHibernatable(
		actor: AnyActorInstance,
		conn: AnyConn,
		state: State,
	): boolean;
}

// MARK: WebSocket
const WEBSOCKET_DRIVER: ConnDriver<ConnDriverWebSocketState> = {
	sendMessage: (
		actor: AnyActorInstance,
		conn: AnyConn,
		state: ConnDriverWebSocketState,
		message: CachedSerializer<protocol.ToClient>,
	) => {
		if (state.websocket.readyState !== ConnReadyState.OPEN) {
			actor.rLog.warn({
				msg: "attempting to send message to closed websocket, this is likely a bug in RivetKit",
				connId: conn.id,
				wsReadyState: state.websocket.readyState,
			});
			return;
		}

		const serialized = message.serialize(state.encoding);

		actor.rLog.debug({
			msg: "sending websocket message",
			encoding: state.encoding,
			dataType: typeof serialized,
			isUint8Array: serialized instanceof Uint8Array,
			isArrayBuffer: serialized instanceof ArrayBuffer,
			dataLength:
				(serialized as any).byteLength || (serialized as any).length,
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
				state.websocket.send(arrayBuffer);
			} else {
				actor.rLog.debug({
					msg: "sending ArrayBuffer",
					byteLength: buffer.byteLength,
				});
				state.websocket.send(buffer);
			}
		} else {
			actor.rLog.debug({
				msg: "sending string data",
				length: (serialized as string).length,
			});
			state.websocket.send(serialized);
		}
	},

	disconnect: async (
		_actor: AnyActorInstance,
		_conn: AnyConn,
		state: ConnDriverWebSocketState,
		reason?: string,
	) => {
		// Close socket
		state.websocket.close(1000, reason);

		// Create promise to wait for socket to close gracefully
		await state.closePromise.promise;
	},

	getConnectionReadyState: (
		_actor: AnyActorInstance,
		_conn: AnyConn,
		state: ConnDriverWebSocketState,
	): ConnReadyState | undefined => {
		return state.websocket.readyState;
	},

	isHibernatable(
		_actor: AnyActorInstance,
		_conn: AnyConn,
		state: ConnDriverWebSocketState,
	): boolean {
		// Extract isHibernatable from the HonoWebSocketAdapter
		if (state.websocket.raw) {
			const raw = state.websocket.raw as HonoWebSocketAdapter;
			if (typeof raw.isHibernatable === "boolean") {
				return raw.isHibernatable;
			}
		}

		return false;
	},
};

// MARK: SSE
const SSE_DRIVER: ConnDriver<ConnDriverSseState> = {
	sendMessage: (
		_actor: AnyActorInstance,
		_conn: AnyConn,
		state: ConnDriverSseState,
		message: CachedSerializer<protocol.ToClient>,
	) => {
		state.stream.writeSSE({
			data: encodeDataToString(message.serialize(state.encoding)),
		});
	},

	disconnect: async (
		_actor: AnyActorInstance,
		_conn: AnyConn,
		state: ConnDriverSseState,
		_reason?: string,
	) => {
		state.stream.close();
	},

	getConnectionReadyState: (
		_actor: AnyActorInstance,
		_conn: AnyConn,
		state: ConnDriverSseState,
	): ConnReadyState | undefined => {
		if (state.stream.aborted || state.stream.closed) {
			return ConnReadyState.CLOSED;
		}

		return ConnReadyState.OPEN;
	},

	isHibernatable(): boolean {
		return false;
	},
};

// MARK: HTTP
const HTTP_DRIVER: ConnDriver<ConnDriverHttpState> = {
	getConnectionReadyState(_actor, _conn) {
		// TODO: This might not be the correct logic
		return ConnReadyState.OPEN;
	},
	disconnect: async () => {
		// Noop
		// TODO: Abort the request
	},
	isHibernatable(): boolean {
		return false;
	},
};

/** List of all connection drivers. */
export const CONN_DRIVERS: Record<ConnDriverKind, ConnDriver<unknown>> = {
	[ConnDriverKind.WEBSOCKET]: WEBSOCKET_DRIVER,
	[ConnDriverKind.SSE]: SSE_DRIVER,
	[ConnDriverKind.HTTP]: HTTP_DRIVER,
};

export function getConnDriverKindFromState(
	state: ConnDriverState,
): ConnDriverKind {
	if (ConnDriverKind.WEBSOCKET in state) return ConnDriverKind.WEBSOCKET;
	else if (ConnDriverKind.SSE in state) return ConnDriverKind.SSE;
	else if (ConnDriverKind.HTTP in state) return ConnDriverKind.HTTP;
	else assertUnreachable(state);
}
