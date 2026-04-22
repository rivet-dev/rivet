import type { JsNativeDatabase, JsKvEntry, JsKvListOptions } from "./index";

export type { JsNativeDatabase, JsKvEntry, JsKvListOptions };

// Re-export protocol types from the envoy protocol package
export * as protocol from "@rivetkit/engine-envoy-protocol";

export interface HibernatingWebSocketMetadata {
	gatewayId: ArrayBuffer;
	requestId: ArrayBuffer;
	envoyMessageIndex: number;
	rivetMessageIndex: number;
	path: string;
	headers: Record<string, string>;
}

export interface KvListOptions {
	reverse?: boolean;
	limit?: number;
}

/** Matches the TS EnvoyHandle interface from @rivetkit/engine-envoy-client */
export interface EnvoyHandle {
	shutdown(immediate: boolean): void;
	getProtocolMetadata(): any | undefined;
	getEnvoyKey(): string;
	started(): Promise<void>;
	getActor(actorId: string, generation?: number): any | undefined;
	sleepActor(actorId: string, generation?: number): void;
	stopActor(actorId: string, generation?: number, error?: string): void;
	destroyActor(actorId: string, generation?: number): void;
	setAlarm(
		actorId: string,
		alarmTs: number | null,
		generation?: number,
	): void;
	kvGet(actorId: string, keys: Uint8Array[]): Promise<(Uint8Array | null)[]>;
	kvListAll(
		actorId: string,
		options?: KvListOptions,
	): Promise<[Uint8Array, Uint8Array][]>;
	kvListRange(
		actorId: string,
		start: Uint8Array,
		end: Uint8Array,
		exclusive?: boolean,
		options?: KvListOptions,
	): Promise<[Uint8Array, Uint8Array][]>;
	kvListPrefix(
		actorId: string,
		prefix: Uint8Array,
		options?: KvListOptions,
	): Promise<[Uint8Array, Uint8Array][]>;
	kvPut(actorId: string, entries: [Uint8Array, Uint8Array][]): Promise<void>;
	kvDelete(actorId: string, keys: Uint8Array[]): Promise<void>;
	kvDeleteRange(
		actorId: string,
		start: Uint8Array,
		end: Uint8Array,
	): Promise<void>;
	kvDrop(actorId: string): Promise<void>;
	restoreHibernatingRequests(
		actorId: string,
		metaEntries: HibernatingWebSocketMetadata[],
	): void;
	sendHibernatableWebSocketMessageAck(
		gatewayId: ArrayBuffer,
		requestId: ArrayBuffer,
		clientMessageIndex: number,
	): void;
	startServerlessActor(payload: ArrayBuffer): Promise<void>;
}

/** Matches the TS EnvoyConfig interface from @rivetkit/engine-envoy-client */
export interface EnvoyConfig {
	logger?: any;
	version: number;
	endpoint: string;
	token?: string;
	namespace: string;
	poolName: string;
	prepopulateActorNames: Record<string, { metadata: Record<string, any> }>;
	metadata?: Record<string, any>;
	notGlobal?: boolean;
	debugLatencyMs?: number;
	serverlessStartPayload?: ArrayBuffer;
	fetch: (
		envoyHandle: EnvoyHandle,
		actorId: string,
		gatewayId: ArrayBuffer,
		requestId: ArrayBuffer,
		request: Request,
	) => Promise<Response>;
	websocket: (
		envoyHandle: EnvoyHandle,
		actorId: string,
		ws: any,
		gatewayId: ArrayBuffer,
		requestId: ArrayBuffer,
		request: Request,
		path: string,
		headers: Record<string, string>,
		isHibernatable: boolean,
		isRestoringHibernatable: boolean,
	) => Promise<void>;
	hibernatableWebSocket: {
		canHibernate: (
			actorId: string,
			gatewayId: ArrayBuffer,
			requestId: ArrayBuffer,
			request: Request,
		) => boolean;
	};
	onActorStart: (
		envoyHandle: EnvoyHandle,
		actorId: string,
		generation: number,
		config: import("@rivetkit/engine-envoy-protocol").ActorConfig,
		preloadedKv:
			| import("@rivetkit/engine-envoy-protocol").PreloadedKv
			| null,
		sqliteSchemaVersion: number,
		sqliteStartupData:
			| import("@rivetkit/engine-envoy-protocol").SqliteStartupData
			| null,
	) => Promise<void>;
	onActorStop: (
		envoyHandle: EnvoyHandle,
		actorId: string,
		generation: number,
		reason: import("@rivetkit/engine-envoy-protocol").StopActorReason,
	) => Promise<void>;
	onShutdown: () => void;
}

/** Start the native envoy synchronously. Returns a handle immediately. */
export declare function startEnvoySync(config: EnvoyConfig): EnvoyHandle;

/** Start the native envoy and wait for it to be ready. */
export declare function startEnvoy(config: EnvoyConfig): Promise<EnvoyHandle>;

/** Open a native database backed by envoy KV for the specified actor. */
export declare function openDatabaseFromEnvoy(
	handle: EnvoyHandle,
	actorId: string,
): Promise<JsNativeDatabase>;
export declare const utils: {};
