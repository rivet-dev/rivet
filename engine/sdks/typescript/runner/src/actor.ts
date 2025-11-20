import type * as protocol from "@rivetkit/engine-runner-protocol";
import type { PendingRequest } from "./tunnel";
import type { WebSocketTunnelAdapter } from "./websocket-tunnel-adapter";
import { arraysEqual, promiseWithResolvers } from "./utils";
import { logger } from "./log";
import * as tunnelId from "./tunnel-id";

export interface ActorConfig {
	name: string;
	key: string | null;
	createTs: bigint;
	input: Uint8Array | null;
}

export class RunnerActor {
	actorId: string;
	generation: number;
	config: ActorConfig;
	pendingRequests: Array<{
		gatewayId: protocol.GatewayId;
		requestId: protocol.RequestId;
		request: PendingRequest;
	}> = [];
	webSockets: Array<{
		gatewayId: protocol.GatewayId;
		requestId: protocol.RequestId;
		ws: WebSocketTunnelAdapter;
	}> = [];
	actorStartPromise: ReturnType<typeof promiseWithResolvers<void>>;

	/**
	 * If restoreHibernatingRequests has been called. This is used to assert
	 * that the caller is implemented correctly.
	 **/
	hibernationRestored: boolean = false;

	constructor(
		actorId: string,
		generation: number,
		config: ActorConfig,
		/**
		 * List of hibernating requests provided by the gateway on actor start.
		 * This represents the WebSocket connections that the gateway knows about.
		 **/
		public hibernatingRequests: readonly protocol.HibernatingRequest[],
	) {
		this.actorId = actorId;
		this.generation = generation;
		this.config = config;
		this.actorStartPromise = promiseWithResolvers();
	}

	// Pending request methods
	getPendingRequest(
		gatewayId: protocol.GatewayId,
		requestId: protocol.RequestId,
	): PendingRequest | undefined {
		return this.pendingRequests.find(
			(entry) =>
				arraysEqual(entry.gatewayId, gatewayId) &&
				arraysEqual(entry.requestId, requestId),
		)?.request;
	}

	createPendingRequest(
		gatewayId: protocol.GatewayId,
		requestId: protocol.RequestId,
		clientMessageIndex: number,
	) {
		const exists =
			this.getPendingRequest(gatewayId, requestId) !== undefined;
		if (exists) {
			logger()?.warn({
				msg: "attempting to set pending request twice, replacing existing",
				gatewayId: tunnelId.gatewayIdToString(gatewayId),
				requestId: tunnelId.requestIdToString(requestId),
			});
			// Delete existing pending request before adding the new one
			this.deletePendingRequest(gatewayId, requestId);
		}
		this.pendingRequests.push({
			gatewayId,
			requestId,
			request: {
				resolve: () => {},
				reject: () => {},
				actorId: this.actorId,
				gatewayId: gatewayId,
				requestId: requestId,
				clientMessageIndex,
			},
		});
		logger()?.debug({
			msg: "added pending request",
			gatewayId: tunnelId.gatewayIdToString(gatewayId),
			requestId: tunnelId.requestIdToString(requestId),
			length: this.pendingRequests.length,
		});
	}

	createPendingRequestWithStreamController(
		gatewayId: protocol.GatewayId,
		requestId: protocol.RequestId,
		clientMessageIndex: number,
		streamController: ReadableStreamDefaultController<Uint8Array>,
	) {
		const exists =
			this.getPendingRequest(gatewayId, requestId) !== undefined;
		if (exists) {
			logger()?.warn({
				msg: "attempting to set pending request twice, replacing existing",
				gatewayId: tunnelId.gatewayIdToString(gatewayId),
				requestId: tunnelId.requestIdToString(requestId),
			});
			// Delete existing pending request before adding the new one
			this.deletePendingRequest(gatewayId, requestId);
		}
		this.pendingRequests.push({
			gatewayId,
			requestId,
			request: {
				resolve: () => {},
				reject: () => {},
				actorId: this.actorId,
				gatewayId: gatewayId,
				requestId: requestId,
				clientMessageIndex,
				streamController,
			},
		});
		logger()?.debug({
			msg: "added pending request with stream controller",
			gatewayId: tunnelId.gatewayIdToString(gatewayId),
			requestId: tunnelId.requestIdToString(requestId),
			length: this.pendingRequests.length,
		});
	}

	deletePendingRequest(
		gatewayId: protocol.GatewayId,
		requestId: protocol.RequestId,
	) {
		const index = this.pendingRequests.findIndex(
			(entry) =>
				arraysEqual(entry.gatewayId, gatewayId) &&
				arraysEqual(entry.requestId, requestId),
		);
		if (index !== -1) {
			this.pendingRequests.splice(index, 1);
			logger()?.debug({
				msg: "removed pending request",
				gatewayId: tunnelId.gatewayIdToString(gatewayId),
				requestId: tunnelId.requestIdToString(requestId),
				length: this.pendingRequests.length,
			});
		}
	}

	// WebSocket methods
	getWebSocket(
		gatewayId: protocol.GatewayId,
		requestId: protocol.RequestId,
	): WebSocketTunnelAdapter | undefined {
		return this.webSockets.find(
			(entry) =>
				arraysEqual(entry.gatewayId, gatewayId) &&
				arraysEqual(entry.requestId, requestId),
		)?.ws;
	}

	setWebSocket(
		gatewayId: protocol.GatewayId,
		requestId: protocol.RequestId,
		ws: WebSocketTunnelAdapter,
	) {
		const exists = this.getWebSocket(gatewayId, requestId) !== undefined;
		if (exists) {
			logger()?.warn({ msg: "attempting to set websocket twice" });
			return;
		}
		this.webSockets.push({ gatewayId, requestId, ws });
	}

	deleteWebSocket(
		gatewayId: protocol.GatewayId,
		requestId: protocol.RequestId,
	) {
		const index = this.webSockets.findIndex(
			(entry) =>
				arraysEqual(entry.gatewayId, gatewayId) &&
				arraysEqual(entry.requestId, requestId),
		);
		if (index !== -1) {
			this.webSockets.splice(index, 1);
		}
	}
}
