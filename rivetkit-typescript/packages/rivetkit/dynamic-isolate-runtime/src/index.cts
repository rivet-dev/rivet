/**
 * Dynamic isolate bootstrap runtime.
 *
 * This file executes inside the sandboxed isolate process. It loads one
 * user supplied ActorDefinition, instantiates a one actor registry, and
 * exposes envelope handlers that the host runtime calls through
 * isolated-vm references.
 *
 * Bridge direction:
 * - Host to isolate: host invokes exported envelope handlers in this file.
 * - Isolate to host: this file calls host bridge references for KV, alarms,
 *   inline client calls, websocket dispatch, and lifecycle requests.
 */
import {
	HibernatableWebSocketAckState,
} from "../../src/actor/conn/hibernatable-websocket-ack-state";
import { CONN_STATE_MANAGER_SYMBOL } from "../../src/actor/conn/mod";
import { createActorRouter } from "../../src/actor/router";
import { routeWebSocket } from "../../src/actor/router-websocket-endpoints";
import { InlineWebSocketAdapter } from "../../src/common/inline-websocket-adapter";
import {
	DYNAMIC_BOOTSTRAP_CONFIG_GLOBAL_KEY,
	DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS,
	type DynamicBootstrapConfig,
	type DynamicBootstrapExports,
	type DynamicClientCallInput,
	type DynamicHibernatingWebSocketMetadata,
	type FetchEnvelopeInput,
	type FetchEnvelopeOutput,
	type IsolateDispatchPayload,
	type WebSocketCloseEnvelopeInput,
	type WebSocketOpenEnvelopeInput,
	type WebSocketSendEnvelopeInput,
} from "../../src/dynamic/runtime-bridge";
import { RegistryConfigSchema } from "../../src/registry/config";

interface IsolateReferenceLike {
	applySyncPromise(
		receiver: unknown,
		args: unknown[],
		options?: Record<string, unknown>,
	): unknown;
	applySync(
		receiver: unknown,
		args: unknown[],
		options?: Record<string, unknown>,
	): unknown;
}

interface IsolateExternalCopyLike {
	copy(): unknown;
}

interface DynamicHostBridge {
	kvBatchPut: IsolateReferenceLike;
	kvBatchGet: IsolateReferenceLike;
	kvBatchDelete: IsolateReferenceLike;
	kvListPrefix: IsolateReferenceLike;
	setAlarm: IsolateReferenceLike;
	clientCall: IsolateReferenceLike;
	ackHibernatableWebSocketMessage: IsolateReferenceLike;
	startSleep: IsolateReferenceLike;
	startDestroy: IsolateReferenceLike;
	dispatch: IsolateReferenceLike;
	log?: IsolateReferenceLike;
}

interface DynamicHibernatableConnData {
	gatewayId: Uint8Array | ArrayBuffer;
	requestId: Uint8Array | ArrayBuffer;
	serverMessageIndex: number;
	clientMessageIndex: number;
	requestPath: string;
	requestHeaders: Record<string, string>;
}

interface DynamicConnLike {
	id?: string;
}

interface DynamicConnStateManagerLike {
	hibernatableData?: DynamicHibernatableConnData;
}

interface DynamicActorDriver {
	loadActor(actorId: string): Promise<DynamicActorInstanceLike>;
	getContext(actorId: string): unknown;
	kvBatchPut(actorId: string, entries: Array<[Uint8Array, Uint8Array]>): Promise<void>;
	kvBatchGet(actorId: string, keys: Uint8Array[]): Promise<Array<Uint8Array | null>>;
	kvBatchDelete(actorId: string, keys: Uint8Array[]): Promise<void>;
	kvListPrefix(
		actorId: string,
		prefix: Uint8Array,
	): Promise<Array<[Uint8Array, Uint8Array]>>;
	setAlarm(actor: { id: string }, timestamp: number): Promise<void>;
	startSleep(actorId: string): void;
	startDestroy(actorId: string): void;
	onCreateConn?(conn: DynamicConnLike): void;
	onBeforePersistConn?(conn: DynamicConnLike): void;
	onAfterPersistConn?(conn: DynamicConnLike): void;
	onDestroyConn?(conn: DynamicConnLike): void;
}

interface DynamicActorDefinitionLike {
	config: unknown;
	instantiate: () => DynamicActorInstanceLike;
}

interface DynamicActorInstanceLike {
	id: string;
	isStopping: boolean;
	start: (
		actorDriver: DynamicActorDriver,
		inlineClient: unknown,
		actorId: string,
		actorName: string,
		actorKey: unknown,
		region: string,
	) => Promise<void>;
	onAlarm: () => Promise<void>;
	onStop: (mode: "sleep" | "destroy") => Promise<void>;
	conns: Map<string, DynamicConnLike>;
}

interface ResponseLike {
	status?: number;
	headers?: Headers;
	body?: unknown;
	arrayBuffer?: () => Promise<ArrayBufferLike>;
	bytes?: () => Promise<Uint8Array>;
	text?: () => Promise<string>;
}

interface DynamicMessageEvent extends MessageEvent {
	rivetMessageIndex?: number;
}

interface DynamicErrorEvent extends Event {
	message?: string;
}

function readConnStateManager(
	conn: DynamicConnLike,
	stateManagerSymbol: symbol,
): DynamicConnStateManagerLike | undefined {
	const stateManager = (conn as Record<symbol, unknown>)[stateManagerSymbol];
	if (!stateManager || typeof stateManager !== "object") {
		return undefined;
	}
	return stateManager as DynamicConnStateManagerLike;
}

function hasReadableStreamBody(
	body: unknown,
): body is ReadableStream<Uint8Array> {
	if (!body || typeof body !== "object") {
		return false;
	}
	return typeof (body as ReadableStream<Uint8Array>).getReader === "function";
}

const globalObject = globalThis as unknown as Record<string, unknown>;

const bootstrapConfig = readBootstrapConfig();
const hostBridge = readHostBridge();

let loadedActor: DynamicActorInstanceLike | undefined;
let loadingActorPromise: Promise<void> | undefined;
let runtimeStatePromise: Promise<DynamicRuntimeState> | undefined;
const webSocketSessions = new Map<
	number,
	{
		ws: WebSocket;
		adapter: {
			dispatchClientMessageWithMetadata?: (
				payload: string | Buffer,
				messageIndex?: number,
			) => void;
		};
	}
>();
const hibernatableWebSocketAckState = new HibernatableWebSocketAckState();
const CLIENT_ACCESSOR_METHODS = new Set(["get", "getOrCreate", "getForId", "create"]);

type DynamicActorRouter = ReturnType<typeof createActorRouter>;

interface DynamicRuntimeState {
	actorDefinition: DynamicActorDefinitionLike;
	config: unknown;
	actorRouter: DynamicActorRouter;
}

function readBootstrapConfig(): DynamicBootstrapConfig {
	const value = globalObject[DYNAMIC_BOOTSTRAP_CONFIG_GLOBAL_KEY];
	if (!value || typeof value !== "object") {
		throw new Error("dynamic runtime bootstrap config is missing");
	}

	const configValue = value as Partial<DynamicBootstrapConfig>;
	if (
		typeof configValue.actorId !== "string" ||
		typeof configValue.actorName !== "string" ||
		!Array.isArray(configValue.actorKey) ||
		typeof configValue.sourceEntry !== "string" ||
		(configValue.sourceFormat !== "commonjs-js" &&
			configValue.sourceFormat !== "esm-js")
	) {
		throw new Error("dynamic runtime bootstrap config is invalid");
	}

	return {
		actorId: configValue.actorId,
		actorName: configValue.actorName,
		actorKey: configValue.actorKey,
		sourceEntry: configValue.sourceEntry,
		sourceFormat: configValue.sourceFormat,
	};
}

function getRequiredHostRef(key: string): IsolateReferenceLike {
	const value = globalObject[key];
	if (!value || typeof value !== "object") {
		throw new Error(`dynamic runtime host bridge ref is missing: ${key}`);
	}

	const ref = value as Partial<IsolateReferenceLike>;
	if (typeof ref.applySync !== "function") {
		throw new Error(`dynamic runtime host bridge ref is invalid: ${key}`);
	}
	if (typeof ref.applySyncPromise !== "function") {
		throw new Error(
			`dynamic runtime host bridge async ref is invalid: ${key}`,
		);
	}
	return ref as IsolateReferenceLike;
}

function getOptionalHostRef(key: string): IsolateReferenceLike | undefined {
	const value = globalObject[key];
	if (!value || typeof value !== "object") {
		return undefined;
	}

	const ref = value as Partial<IsolateReferenceLike>;
	if (typeof ref.applySync !== "function") {
		return undefined;
	}
	return ref as IsolateReferenceLike;
}

function readHostBridge(): DynamicHostBridge {
	return {
		kvBatchPut: getRequiredHostRef(DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.kvBatchPut),
		kvBatchGet: getRequiredHostRef(DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.kvBatchGet),
		kvBatchDelete: getRequiredHostRef(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.kvBatchDelete,
		),
		kvListPrefix: getRequiredHostRef(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.kvListPrefix,
		),
		setAlarm: getRequiredHostRef(DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.setAlarm),
		clientCall: getRequiredHostRef(DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.clientCall),
		ackHibernatableWebSocketMessage: getRequiredHostRef(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.ackHibernatableWebSocketMessage,
		),
		startSleep: getRequiredHostRef(DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.startSleep),
		startDestroy: getRequiredHostRef(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.startDestroy,
		),
		dispatch: getRequiredHostRef(DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.dispatch),
		log: getOptionalHostRef(DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.log),
	};
}

function resolveSourceSpecifier(): string {
	if (bootstrapConfig.sourceEntry.startsWith("./")) {
		return bootstrapConfig.sourceEntry;
	}
	return `./${bootstrapConfig.sourceEntry}`;
}

async function loadActorDefinition(): Promise<DynamicActorDefinitionLike> {
	const sourceSpecifier = resolveSourceSpecifier();
	let actorModule: unknown;
	try {
		if (bootstrapConfig.sourceFormat === "esm-js") {
			actorModule = await import(sourceSpecifier);
		} else {
			actorModule = require(sourceSpecifier);
		}
	} catch (error) {
		const details = error instanceof Error && error.stack
			? error.stack
			: String(error);
		throw new Error(
			`dynamic runtime failed to load source module (${bootstrapConfig.sourceFormat}): ${details}`,
		);
	}

	const actorDefinition =
		((actorModule as Record<string, unknown>)?.default as unknown) ?? actorModule;
	if (
		!actorDefinition ||
		typeof (actorDefinition as DynamicActorDefinitionLike).instantiate !== "function"
	) {
		throw new Error("dynamic source module must default-export an ActorDefinition");
	}
	return actorDefinition as DynamicActorDefinitionLike;
}

async function getRuntimeState(): Promise<DynamicRuntimeState> {
	if (!runtimeStatePromise) {
		runtimeStatePromise = (async () => {
			const actorDefinition = await loadActorDefinition();
			// Parse directly through the schema so we do not instantiate Registry.
			// Registry constructor auto-starts a runtime on next tick in non-test
			// environments, which pulls in default drivers and is not needed here.
			const config = RegistryConfigSchema.parse({
				use: {
					[bootstrapConfig.actorName]: actorDefinition,
				},
				serveManager: false,
				noWelcome: true,
				test: { enabled: false },
			});
			const actorRouter = createActorRouter(
				config,
				actorDriver,
				undefined,
				false,
			);
			return {
				actorDefinition,
				config,
				actorRouter,
			};
		})();
	}
	return await runtimeStatePromise;
}

function dynamicHostLog(level: "debug" | "warn", message: string): void {
	if (!hostBridge.log) {
		return;
	}

	try {
		hostBridge.log.applySync(undefined, [level, String(message)]);
	} catch {
		// noop
	}
}

function bridgeCall<T>(ref: IsolateReferenceLike, args: unknown[]): Promise<T> {
	// Use applySyncPromise so the isolate can synchronously hand control back
	// to the host and still await a promise result. We only pass structured
	// clone safe values with copy semantics.
	const result = ref.applySyncPromise(undefined, args, {
		arguments: {
			copy: true,
		},
	});

	if (
		result &&
		typeof result === "object" &&
		typeof (result as IsolateExternalCopyLike).copy === "function"
	) {
		return Promise.resolve((result as IsolateExternalCopyLike).copy() as T);
	}

	return Promise.resolve(result as T);
}

function bridgeCallSync<T>(ref: IsolateReferenceLike, args: unknown[]): T {
	// Use applySync for fire and forget bridge calls that must complete in the
	// current turn, such as dispatch and lifecycle signals.
	return ref.applySync(undefined, args, {
		arguments: {
			copy: true,
		},
	}) as T;
}

function toArrayBuffer(input: Uint8Array | ArrayBuffer): ArrayBuffer {
	if (input instanceof ArrayBuffer) {
		return input;
	}
	return input.buffer.slice(
		input.byteOffset,
		input.byteOffset + input.byteLength,
	) as ArrayBuffer;
}

function toArrayBufferFromArrayBufferLike(input: ArrayBufferLike): ArrayBuffer {
	if (input instanceof ArrayBuffer) {
		return input.slice(0);
	}
	return new Uint8Array(input).slice().buffer;
}

function toBuffer(input: ArrayBuffer): Buffer {
	return Buffer.from(new Uint8Array(input));
}

function responseHeadersToEntries(headers: Headers | Record<string, string> | undefined): Array<
	[string, string]
> {
	if (!headers) {
		return [];
	}
	if (typeof (headers as Headers).forEach === "function") {
		const entries: Array<[string, string]> = [];
		(headers as Headers).forEach((value, key) => {
			entries.push([key, value]);
		});
		return entries;
	}
	return Object.entries(headers).map(([key, value]) => [String(key), String(value)]);
}

async function responseBodyToBinary(
	response: ResponseLike | undefined,
): Promise<ArrayBuffer> {
	if (!response) {
		return new ArrayBuffer(0);
	}
	if (typeof response.arrayBuffer === "function") {
		return toArrayBufferFromArrayBufferLike(await response.arrayBuffer());
	}
	if (typeof response.bytes === "function") {
		return toArrayBuffer(await response.bytes());
	}

	const bodyValue = response.body;
	if (bodyValue !== undefined && bodyValue !== null) {
		if (hasReadableStreamBody(bodyValue)) {
			const reader = bodyValue.getReader();
			const chunks: Uint8Array[] = [];
			let totalLength = 0;
			while (true) {
				const result = await reader.read();
				if (result.done) {
					break;
				}
				const chunk = result.value instanceof Uint8Array
					? result.value
					: new Uint8Array(result.value);
				chunks.push(chunk);
				totalLength += chunk.byteLength;
			}
			const merged = new Uint8Array(totalLength);
			let offset = 0;
			for (const chunk of chunks) {
				merged.set(chunk, offset);
				offset += chunk.byteLength;
			}
			return toArrayBuffer(merged);
		}
		if (typeof bodyValue === "string") {
			return toArrayBuffer(Buffer.from(bodyValue, "utf8"));
		}
		if (Array.isArray(bodyValue)) {
			return toArrayBuffer(Buffer.from(bodyValue));
		}
		if (bodyValue instanceof Uint8Array) {
			return toArrayBuffer(bodyValue);
		}
		if (ArrayBuffer.isView(bodyValue)) {
			const view = new Uint8Array(
				bodyValue.buffer,
				bodyValue.byteOffset,
				bodyValue.byteLength,
			);
			return toArrayBuffer(view);
		}
		if (bodyValue instanceof ArrayBuffer) {
			return bodyValue.slice(0);
		}
	}

	const privateBody = (response as Record<string, unknown>)._body;
	if (privateBody !== undefined && privateBody !== null) {
		if (privateBody instanceof Uint8Array) {
			return toArrayBuffer(privateBody);
		}
		if (privateBody instanceof ArrayBuffer) {
			return privateBody.slice(0);
		}
		if (ArrayBuffer.isView(privateBody)) {
			const view = new Uint8Array(
				privateBody.buffer,
				privateBody.byteOffset,
				privateBody.byteLength,
			);
			return toArrayBuffer(view);
		}
		if (Array.isArray(privateBody)) {
			return toArrayBuffer(Buffer.from(privateBody));
		}
	}

	if (typeof response.text === "function") {
		const text: string = await response.text();
		const contentType =
			response?.headers && typeof response.headers.get === "function"
				? response.headers.get("content-type") ?? ""
				: "";
		if (!contentType.includes("application/json")) {
			const trimmedText = text.trim();
			// Some sandbox response shims stringify Uint8Array bodies as "1,2,3".
			const numericTokens = trimmedText
				.split(/[^\d]+/u)
				.filter((value: string) => value.length > 0);
			if (numericTokens.length > 1) {
				const bytes = numericTokens.map((value: string) =>
					Number.parseInt(value, 10),
				);
				if (
					bytes.every(
						(value: number) =>
							Number.isInteger(value) && value >= 0 && value <= 255,
					)
				) {
					return toArrayBuffer(Buffer.from(bytes));
				}
			}
			return toArrayBuffer(Buffer.from(text, "latin1"));
		}
		return toArrayBuffer(Buffer.from(text, "utf8"));
	}
	return bodyValue === undefined || bodyValue === null
		? new ArrayBuffer(0)
		: toArrayBuffer(Buffer.from(String(bodyValue), "utf8"));
}

async function loadActor(requestActorId: string): Promise<DynamicActorInstanceLike> {
	if (requestActorId !== bootstrapConfig.actorId) {
		throw new Error("dynamic actor runtime received unexpected actor id");
	}

	if (loadedActor && !loadedActor.isStopping) {
		return loadedActor;
	}
	if (loadingActorPromise) {
		await loadingActorPromise;
		if (loadedActor) return loadedActor;
	}

	loadingActorPromise = (async () => {
		const { actorDefinition } = await getRuntimeState();
		const actor = actorDefinition.instantiate();
		try {
			await actor.start(
				actorDriver,
				inlineClient,
				bootstrapConfig.actorId,
				bootstrapConfig.actorName,
				bootstrapConfig.actorKey,
				"unknown",
			);
			loadedActor = actor;
		} catch (error) {
			dynamicHostLog(
				"warn",
				"actor.start failed: " +
					(error instanceof Error && error.stack
						? error.stack
						: String(error)),
			);
			throw error;
		}
	})();
	await loadingActorPromise;
	loadingActorPromise = undefined;
	if (!loadedActor) {
		throw new Error("failed to load actor");
	}
	return loadedActor;
}

function createClientHandleProxy(
	actorName: string,
	accessorMethod: DynamicClientCallInput["accessorMethod"],
	accessorArgs: unknown[],
): object {
	return new Proxy(
		{},
		{
			get(_target, operation) {
				if (operation === "then") {
					return undefined;
				}
				if (typeof operation !== "string") {
					return undefined;
				}
				return (...operationArgs: unknown[]) =>
					bridgeCall(hostBridge.clientCall, [
						{
							actorName,
							accessorMethod,
							accessorArgs,
							operation,
							operationArgs,
						} satisfies DynamicClientCallInput,
					]);
			},
		},
	);
}

const inlineClient = new Proxy(
	{},
	{
		get(_target, actorName) {
			if (typeof actorName !== "string") {
				return undefined;
			}
			return new Proxy(
				{},
				{
					get(_accessorTarget, accessorMethod) {
						if (
							typeof accessorMethod !== "string" ||
							!CLIENT_ACCESSOR_METHODS.has(accessorMethod)
						) {
							return undefined;
						}
						return (...accessorArgs: unknown[]) =>
							createClientHandleProxy(
								actorName,
								accessorMethod as DynamicClientCallInput["accessorMethod"],
								accessorArgs,
							);
					},
				},
			);
		},
	},
);

const actorDriver: DynamicActorDriver = {
	async loadActor(requestActorId: string): Promise<DynamicActorInstanceLike> {
		return await loadActor(requestActorId);
	},
	getContext(_actorId: string): Record<string, never> {
		return {};
	},
	async kvBatchPut(
		actorIdValue: string,
		entries: Array<[Uint8Array, Uint8Array]>,
	): Promise<void> {
		const encoded = entries.map(([key, value]) => [
			toArrayBuffer(key),
			toArrayBuffer(value),
		]);
		await bridgeCall(hostBridge.kvBatchPut, [actorIdValue, encoded]);
	},
	async kvBatchGet(
		actorIdValue: string,
		keys: Uint8Array[],
	): Promise<Array<Uint8Array | null>> {
		const encodedKeys = keys.map((key) => toArrayBuffer(key));
		const values = await bridgeCall<Array<ArrayBuffer | null>>(hostBridge.kvBatchGet, [
			actorIdValue,
			encodedKeys,
		]);
		return values.map((value) =>
			value === null ? null : new Uint8Array(value)
		);
	},
	async kvBatchDelete(actorIdValue: string, keys: Uint8Array[]): Promise<void> {
		const encodedKeys = keys.map((key) => toArrayBuffer(key));
		await bridgeCall(hostBridge.kvBatchDelete, [actorIdValue, encodedKeys]);
	},
	async kvListPrefix(
		actorIdValue: string,
		prefix: Uint8Array,
	): Promise<Array<[Uint8Array, Uint8Array]>> {
		const encodedPrefix = toArrayBuffer(prefix);
		const values = await bridgeCall<Array<[ArrayBuffer, ArrayBuffer]>>(
			hostBridge.kvListPrefix,
			[
				actorIdValue,
				encodedPrefix,
			],
		);
		return values.map(([key, value]) => [new Uint8Array(key), new Uint8Array(value)]);
	},
	async setAlarm(actor, timestamp: number): Promise<void> {
		await bridgeCall(hostBridge.setAlarm, [actor.id, timestamp]);
	},
	startSleep(requestActorId: string): void {
		bridgeCallSync(hostBridge.startSleep, [requestActorId]);
	},
	startDestroy(requestActorId: string): void {
		bridgeCallSync(hostBridge.startDestroy, [requestActorId]);
	},
	onCreateConn(conn: DynamicConnLike): void {
		const connStateManager = readConnStateManager(
			conn,
			CONN_STATE_MANAGER_SYMBOL,
		);
		const hibernatable = connStateManager?.hibernatableData;
		if (!hibernatable || typeof conn?.id !== "string") {
			return;
		}

		const serverMessageIndex = Number(hibernatable.serverMessageIndex);
		if (!Number.isFinite(serverMessageIndex)) {
			return;
		}

		hibernatableWebSocketAckState.createConnEntry(
			conn.id,
			serverMessageIndex,
		);
	},
	onBeforePersistConn(conn: DynamicConnLike): void {
		const connStateManager = readConnStateManager(
			conn,
			CONN_STATE_MANAGER_SYMBOL,
		);
		const hibernatable = connStateManager?.hibernatableData;
		if (!hibernatable || typeof conn?.id !== "string") {
			return;
		}

		const serverMessageIndex = Number(hibernatable.serverMessageIndex);
		if (!Number.isFinite(serverMessageIndex)) {
			return;
		}

		if (!hibernatableWebSocketAckState.hasConnEntry(conn.id)) {
			hibernatableWebSocketAckState.createConnEntry(
				conn.id,
				serverMessageIndex - 1,
			);
		}

		hibernatableWebSocketAckState.onBeforePersist(conn.id, serverMessageIndex);
	},
	onAfterPersistConn(conn: DynamicConnLike): void {
		try {
			const connStateManager = readConnStateManager(
				conn,
				CONN_STATE_MANAGER_SYMBOL,
			);
			const hibernatable = connStateManager?.hibernatableData;
			if (!hibernatable) {
				return;
			}

			const connId = conn?.id;
			if (typeof connId !== "string") {
				return;
			}

			if (!hibernatableWebSocketAckState.hasConnEntry(connId)) {
				return;
			}

			const serverMessageIndex = hibernatableWebSocketAckState.consumeAck(connId);
			if (serverMessageIndex === undefined) {
				return;
			}

			bridgeCallSync(hostBridge.ackHibernatableWebSocketMessage, [
				toArrayBuffer(hibernatable.gatewayId),
				toArrayBuffer(hibernatable.requestId),
				serverMessageIndex,
			]);
		} catch (error) {
			const details = error instanceof Error && error.stack
				? error.stack
				: String(error);
			dynamicHostLog(
				"warn",
				"failed to ack hibernatable websocket message: " + details,
			);
			throw error;
		}
	},
	onDestroyConn(conn: DynamicConnLike): void {
		if (typeof conn?.id === "string") {
			hibernatableWebSocketAckState.deleteConnEntry(conn.id);
		}
	},
};

function ensureRequestArrayBuffer(
	request: Request,
	requestBody: ArrayBuffer | undefined,
): void {
	if (typeof request.arrayBuffer === "function") {
		return;
	}

	const fallbackBody = requestBody ? requestBody.slice(0) : new ArrayBuffer(0);
	Object.defineProperty(request, "arrayBuffer", {
		configurable: true,
		value: async () => fallbackBody.slice(0),
	});
}

async function dynamicFetchEnvelope(
	input: FetchEnvelopeInput,
): Promise<FetchEnvelopeOutput> {
	const request = new Request(input.url, {
		method: input.method,
		headers: input.headers,
		body: input.body,
	});
	ensureRequestArrayBuffer(request, input.body);
	const runtimeState = await getRuntimeState();
	const response = await runtimeState.actorRouter.fetch(request, {
		actorId: bootstrapConfig.actorId,
	});
	const status = typeof response.status === "number" ? response.status : 200;
	const body = await responseBodyToBinary(response);
	if (status >= 500) {
		const preview = toBuffer(body).toString("utf8");
		dynamicHostLog(
			"warn",
			"fetch status >= 500: status=" +
				status +
				" url=" +
				request.url +
				" bodyPreview=" +
				preview,
		);
	}
	return {
		status,
		headers: responseHeadersToEntries(response.headers),
		body,
	};
}

async function dynamicDispatchAlarmEnvelope(): Promise<boolean> {
	const actor = await loadActor(bootstrapConfig.actorId);
	await actor.onAlarm();
	return true;
}

async function dynamicStopEnvelope(mode: "sleep" | "destroy"): Promise<boolean> {
	if (!loadedActor) return true;
	await loadedActor.onStop(mode);
	loadedActor = undefined;
	return true;
}

async function dynamicOpenWebSocketEnvelope(
	input: WebSocketOpenEnvelopeInput,
): Promise<boolean> {
	const headers = input.headers ?? {};
	const requestPath = input.path || "/connect";
	const pathOnly = requestPath.split("?")[0];
	const request = new Request(
		requestPath.startsWith("http") ? requestPath : `http://actor${requestPath}`,
		{ method: "GET", headers },
	);
	const gatewayId = input.gatewayId;
	const requestId = input.requestId;
	const runtimeState = await getRuntimeState();
	const handler = await routeWebSocket(
		request,
		pathOnly,
		headers,
		runtimeState.config,
		actorDriver,
		bootstrapConfig.actorId,
		input.encoding,
		input.params,
		gatewayId,
		requestId,
		Boolean(input.isHibernatable),
		Boolean(input.isRestoringHibernatable),
	);
	const adapter = new InlineWebSocketAdapter(handler);
	const ws = adapter.clientWebSocket;
	webSocketSessions.set(input.sessionId, { ws, adapter });

	ws.addEventListener("open", () => {
		bridgeCallSync<void>(hostBridge.dispatch, [
			{
				type: "open",
				sessionId: input.sessionId,
			} satisfies IsolateDispatchPayload,
		]);
	});
	ws.addEventListener("message", (event: DynamicMessageEvent) => {
		const data = event.data;
		if (typeof data === "string") {
			bridgeCallSync<void>(hostBridge.dispatch, [
				{
					type: "message",
					sessionId: input.sessionId,
					kind: "text",
					text: data,
					rivetMessageIndex: event.rivetMessageIndex,
				} satisfies IsolateDispatchPayload,
			]);
			return;
		}
		if (data instanceof Blob) {
			void data
				.arrayBuffer()
				.then((buffer) => {
					bridgeCallSync<void>(hostBridge.dispatch, [
						{
							type: "message",
							sessionId: input.sessionId,
							kind: "binary",
							data: toArrayBufferFromArrayBufferLike(buffer),
							rivetMessageIndex: event.rivetMessageIndex,
						} satisfies IsolateDispatchPayload,
					]);
				})
				.catch((error) => {
					bridgeCallSync<void>(hostBridge.dispatch, [
						{
							type: "error",
							sessionId: input.sessionId,
							message:
								error instanceof Error ? error.message : String(error),
						} satisfies IsolateDispatchPayload,
					]);
				});
			return;
		}
		if (ArrayBuffer.isView(data)) {
			bridgeCallSync<void>(hostBridge.dispatch, [
				{
					type: "message",
					sessionId: input.sessionId,
					kind: "binary",
					data: toArrayBuffer(
						new Uint8Array(data.buffer, data.byteOffset, data.byteLength),
					),
					rivetMessageIndex: event.rivetMessageIndex,
				} satisfies IsolateDispatchPayload,
			]);
			return;
		}
		if (data instanceof ArrayBuffer) {
			bridgeCallSync<void>(hostBridge.dispatch, [
				{
					type: "message",
					sessionId: input.sessionId,
					kind: "binary",
					data: data.slice(0),
					rivetMessageIndex: event.rivetMessageIndex,
				} satisfies IsolateDispatchPayload,
			]);
		}
	});
	ws.addEventListener("close", (event: CloseEvent) => {
		webSocketSessions.delete(input.sessionId);
		bridgeCallSync<void>(hostBridge.dispatch, [
			{
				type: "close",
				sessionId: input.sessionId,
				code: event.code,
				reason: event.reason,
				wasClean: event.wasClean,
			} satisfies IsolateDispatchPayload,
		]);
	});
	ws.addEventListener("error", (event: DynamicErrorEvent) => {
		bridgeCallSync<void>(hostBridge.dispatch, [
			{
				type: "error",
				sessionId: input.sessionId,
				message: event?.message || "dynamic websocket error",
			} satisfies IsolateDispatchPayload,
		]);
	});
	return true;
}

async function dynamicWebSocketSendEnvelope(
	input: WebSocketSendEnvelopeInput,
): Promise<boolean> {
	const session = webSocketSessions.get(input.sessionId);
	if (!session) {
		throw new Error(
			`dynamic websocket session not found for send: ${input.sessionId}`,
		);
	}
	const payload =
		input.kind === "text"
			? input.text || ""
			: input.data
				? toBuffer(input.data)
				: undefined;
	if (payload === undefined) {
		throw new Error(
			`dynamic websocket payload missing for session ${input.sessionId}`,
		);
	}
	if (typeof session.adapter.dispatchClientMessageWithMetadata === "function") {
		session.adapter.dispatchClientMessageWithMetadata(
			payload,
			input.rivetMessageIndex,
		);
		return true;
	}
	if (input.rivetMessageIndex !== undefined) {
		throw new Error(
			"inline websocket adapter missing dispatchClientMessageWithMetadata for indexed message dispatch",
		);
	}
	if (input.kind === "text") {
		session.ws.send(input.text || "");
		return true;
	}
	session.ws.send(toBuffer(input.data ?? new ArrayBuffer(0)));
	return true;
}

async function dynamicWebSocketCloseEnvelope(
	input: WebSocketCloseEnvelopeInput,
): Promise<boolean> {
	const session = webSocketSessions.get(input.sessionId);
	if (!session) return false;
	session.ws.close(input.code, input.reason);
	return true;
}

async function dynamicGetHibernatingWebSocketsEnvelope(): Promise<
	Array<DynamicHibernatingWebSocketMetadata>
> {
	const actor = await loadActor(bootstrapConfig.actorId);
	const conns = actor.conns ?? new Map();
	return Array.from(conns.values())
		.map((conn) => {
			const connStateManager = readConnStateManager(
				conn,
				CONN_STATE_MANAGER_SYMBOL,
			);
			const hibernatable = connStateManager?.hibernatableData;
			if (!hibernatable) return undefined;
			return {
				gatewayId: toArrayBuffer(hibernatable.gatewayId),
				requestId: toArrayBuffer(hibernatable.requestId),
				serverMessageIndex: hibernatable.serverMessageIndex,
				clientMessageIndex: hibernatable.clientMessageIndex,
				path: hibernatable.requestPath,
				headers: hibernatable.requestHeaders,
			};
		})
		.filter((entry): entry is DynamicHibernatingWebSocketMetadata => {
			return entry !== undefined;
		});
}

async function dynamicDisposeEnvelope(): Promise<boolean> {
	for (const session of webSocketSessions.values()) {
		try {
			session.ws.close(1001, "dynamic.runtime.disposed");
		} catch {
			// noop
		}
	}
	webSocketSessions.clear();
	return true;
}

const bootstrapExports: DynamicBootstrapExports = {
	dynamicFetchEnvelope,
	dynamicDispatchAlarmEnvelope,
	dynamicStopEnvelope,
	dynamicOpenWebSocketEnvelope,
	dynamicWebSocketSendEnvelope,
	dynamicWebSocketCloseEnvelope,
	dynamicGetHibernatingWebSocketsEnvelope,
	dynamicDisposeEnvelope,
};

module.exports = bootstrapExports;
