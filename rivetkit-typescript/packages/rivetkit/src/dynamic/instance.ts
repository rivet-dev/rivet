import type { BaseActorInstance } from "@/actor/instance/mod";
import type { Encoding } from "@/actor/protocol/serde";
import type { UniversalWebSocket } from "@/common/websocket-interface";
import {
	DynamicActorIsolateRuntime,
	type DynamicWebSocketOpenOptions,
} from "./isolate-runtime";

export class DynamicActorInstance implements BaseActorInstance {
	#actorId: string;
	#runtime: DynamicActorIsolateRuntime;
	#isStopping = false;

	constructor(actorId: string, runtime: DynamicActorIsolateRuntime) {
		this.#actorId = actorId;
		this.#runtime = runtime;
	}

	get id(): string {
		return this.#actorId;
	}

	get isStopping(): boolean {
		return this.#isStopping;
	}

	async onStop(mode: "sleep" | "destroy"): Promise<void> {
		if (this.#isStopping) return;
		this.#isStopping = true;
		try {
			await this.#runtime.stop(mode);
		} finally {
			await this.#runtime.dispose();
		}
	}

	async onAlarm(): Promise<void> {
		await this.#runtime.dispatchAlarm();
	}

	async fetch(request: Request): Promise<Response> {
		return await this.#runtime.fetch(request);
	}

	async openWebSocket(
		path: string,
		encoding: Encoding,
		params: unknown,
		options?: DynamicWebSocketOpenOptions,
	): Promise<UniversalWebSocket> {
		return await this.#runtime.openWebSocket(path, encoding, params, options);
	}

	async getHibernatingWebSockets() {
		return await this.#runtime.getHibernatingWebSockets();
	}

	async forwardIncomingWebSocketMessage(
		websocket: UniversalWebSocket,
		data: string | ArrayBufferLike | Blob | ArrayBufferView,
		rivetMessageIndex?: number,
	): Promise<void> {
		await this.#runtime.forwardIncomingWebSocketMessage(
			websocket,
			data,
			rivetMessageIndex,
		);
	}
}
