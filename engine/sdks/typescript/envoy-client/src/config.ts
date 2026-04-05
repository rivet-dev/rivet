import type { Logger } from "pino";
import * as protocol from "@rivetkit/engine-envoy-protocol";
import type { EnvoyHandle } from "./handle.js";
import { ShutdownReason } from "./utils.js";

export interface EnvoyConfig {
	logger?: Logger;
	version: number;
	endpoint: string;
	token?: string;
	namespace: string;
	poolName: string;
	prepopulateActorNames: Record<string, { metadata: Record<string, any> }>;
	metadata?: Record<string, any>;

	/**
	 * Debug option to inject artificial latency (in ms) into WebSocket
	 * communication. Messages are queued and delivered in order after the
	 * configured delay.
	 *
	 * @experimental For testing only.
	 */
	debugLatencyMs?: number;

	/** Called when receiving a network request. */
	fetch: (
		envoyHandle: EnvoyHandle,
		actorId: string,
		gatewayId: protocol.GatewayId,
		requestId: protocol.RequestId,
		request: Request,
	) => Promise<Response>;

	/** Payload to start an actor from a serverless SSE POST request. Can also use `EnvoyHandle.startServerless` */
	serverlessStartPayload?: ArrayBuffer;

	// TODO: fix doc comment
	/**
	 * Called when receiving a WebSocket connection.
	 *
	 * All event listeners must be added synchronously inside this function or
	 * else events may be missed. The open event will fire immediately after
	 * this function finishes.
	 *
	 * Any errors thrown here will disconnect the WebSocket immediately.
	 *
	 * While `path` and `headers` are partially redundant to the data in the
	 * `Request`, they may vary slightly from the actual content of `Request`.
	 * Prefer to persist the `path` and `headers` properties instead of the
	 * `Request` itself.
	 *
	 * ## Hibernating Web Sockets
	 *
	 * ### Implementation Requirements
	 *
	 * **Requirement 1: Persist HWS Immediately**
	 *
	 * This is responsible for persisting hibernatable WebSockets immediately
	 * (do not wait for open event). It is not time sensitive to flush the
	 * connection state. If this fails to persist the HWS, the client's
	 * WebSocket will be disconnected on next wake in the call to
	 * `Tunnel::restoreHibernatingRequests` since the connection entry will not
	 * exist.
	 *
	 * **Requirement 2: Persist Message Index On `message`**
	 *
	 * In the `message` event listener, this handler must persist the message
	 * index from the event. The request ID is available at
	 * `event.rivetRequestId` and message index at `event.rivetMessageIndex`.
	 *
	 * The message index should not be flushed immediately. Instead, this
	 * should:
	 *
	 * - Debounce calls to persist the message index
	 * - After each persist, call
	 *   `Runner::sendHibernatableWebSocketMessageAck` to acknowledge the
	 *   message
	 *
	 * This mechanism allows us to buffer messages on the gateway so we can
	 * batch-persist events on our end on a given interval.
	 *
	 * If this fails to persist, then the gateway will replay unacked
	 * messages when the actor starts again.
	 *
	 * **Requirement 3: Remove HWS From Storage On `close`**
	 *
	 * This handler should add an event listener for `close` to remove the
	 * connection from storage.
	 *
	 * If the connection remove fails to persist, the close event will be
	 * called again on the next actor start in
	 * `Tunnel::restoreHibernatingRequests` since there will be no request for
	 * the given connection.
	 *
	 * ### Restoring Connections
	 *
	 * The user of this library is responsible for:
	 * 1. Loading all persisted hibernatable WebSocket metadata for an actor
	 * 2. Calling `Runner::restoreHibernatingRequests` with this metadata at
	 *    the end of `onActorStart`
	 *
	 * `restoreHibernatingRequests` will restore all connections and attach
	 * the appropriate event listeners.
	 *
	 * ### No Open Event On Restoration
	 *
	 * When restoring a HWS, the open event will not be called again. It will
	 * go straight to the message or close event.
	 */
	websocket: (
		envoyHandle: EnvoyHandle,
		actorId: string,
		ws: any,
		gatewayId: protocol.GatewayId,
		requestId: protocol.RequestId,
		request: Request,
		path: string,
		headers: Record<string, string>,
		isHibernatable: boolean,
		isRestoringHibernatable: boolean,
	) => Promise<void>;

	hibernatableWebSocket: {
		/**
		 * Determines if a WebSocket can continue to live while an actor goes to
		 * sleep.
		 */
		canHibernate: (
			actorId: string,
			gatewayId: ArrayBuffer,
			requestId: ArrayBuffer,
			request: Request,
		) => boolean;
	};

	// TODO: Fix doc comment
	/**
	 * Called when an actor starts.
	 *
	 * This callback is responsible for:
	 * 1. Initializing the actor instance
	 * 2. Loading all persisted hibernatable WebSocket metadata for this actor
	 * 3. Calling `Runner::restoreHibernatingRequests` with the loaded metadata
	 *    to restore hibernatable WebSocket connections
	 *
	 * The actor should not be marked as "ready" until after
	 * `restoreHibernatingRequests` completes to ensure all hibernatable
	 * connections are fully restored before the actor processes new requests.
	 */
	onActorStart: (
		envoyHandle: EnvoyHandle,
		actorId: string,
		generation: number,
		config: protocol.ActorConfig,
		preloadedKv: protocol.PreloadedKv | null,
	) => Promise<void>;

	onActorStop: (
		envoyHandle: EnvoyHandle,
		actorId: string,
		generation: number,
		reason: protocol.StopActorReason,
	) => Promise<void>;
	onShutdown: (reason: ShutdownReason) => void;
}
