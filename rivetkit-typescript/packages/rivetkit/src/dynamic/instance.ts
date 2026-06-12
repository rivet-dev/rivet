import type { DynamicActorIsolateRuntime } from "./isolate-runtime";

export class DynamicActorInstance {
	constructor(
		public readonly id: string,
		private readonly runtime: DynamicActorIsolateRuntime,
	) {}

	get isStopping(): boolean {
		return this.runtime.isStopping;
	}

	async onStop(mode: "sleep" | "destroy"): Promise<void> {
		await this.runtime.stop(mode);
	}

	async onAlarm(): Promise<void> {
		await this.runtime.onAlarm();
	}

	async cleanupPersistedConnections(reason?: string): Promise<number> {
		return await this.runtime.cleanupPersistedConnections(reason);
	}

	async getHibernatingWebSockets() {
		return await this.runtime.getHibernatingWebSockets();
	}

	getHibernatingWebSocketMetadata() {
		return this.runtime.getHibernatingWebSocketMetadata();
	}
}
