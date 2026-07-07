import type { Client } from "@/client/client";
import type { RegistryConfig } from "@/registry/config";
import type { DynamicActorLoader } from "./internal";

export interface DynamicActorIsolateRuntimeConfig {
	actorId: string;
	actorName: string;
	actorKey: string[];
	input: unknown;
	region: string;
	loader: DynamicActorLoader;
	inlineClient: Client<any>;
	test: RegistryConfig["test"];
}

export interface DynamicHibernatingWebSocketMetadata {
	gatewayId: ArrayBuffer;
	requestId: ArrayBuffer;
	serverMessageIndex: number;
	clientMessageIndex: number;
	path: string;
	headers: Record<string, string>;
}

export class DynamicActorIsolateRuntime {
	#isStopping = false;

	constructor(private readonly config: DynamicActorIsolateRuntimeConfig) {}

	get isStopping(): boolean {
		return this.#isStopping;
	}

	async start(): Promise<void> {
		await this.config.loader({
			key: this.config.actorKey,
			client: async () => this.config.inlineClient,
		});
	}

	async stop(_mode: "sleep" | "destroy"): Promise<void> {
		this.#isStopping = true;
	}

	async dispose(): Promise<void> {
		this.#isStopping = true;
	}

	async onAlarm(): Promise<void> {}

	async cleanupPersistedConnections(_reason?: string): Promise<number> {
		return 0;
	}

	async getHibernatingWebSockets(): Promise<
		DynamicHibernatingWebSocketMetadata[]
	> {
		return [];
	}

	getHibernatingWebSocketMetadata(): DynamicHibernatingWebSocketMetadata[] {
		return [];
	}
}
