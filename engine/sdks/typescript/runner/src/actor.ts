import type * as protocol from "@rivetkit/engine-runner-protocol";
import type { PendingRequest } from "./tunnel";
import type { WebSocketTunnelAdapter } from "./websocket-tunnel-adapter";
import { arraysEqual } from "./utils";

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

	constructor(actorId: string, generation: number, config: ActorConfig) {
		this.actorId = actorId;
		this.generation = generation;
		this.config = config;
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

	setPendingRequest(
		gatewayId: protocol.GatewayId,
		requestId: protocol.RequestId,
		request: PendingRequest,
	) {
		this.deletePendingRequest(gatewayId, requestId);
		this.pendingRequests.push({ gatewayId, requestId, request });
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
		this.deleteWebSocket(gatewayId, requestId);
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
