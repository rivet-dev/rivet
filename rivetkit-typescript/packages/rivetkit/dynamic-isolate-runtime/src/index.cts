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
import { CONN_STATE_MANAGER_SYMBOL } from "../../src/actor/conn/mod";
import { createRawRequestDriver } from "../../src/actor/conn/drivers/raw-request";
import * as errors from "../../src/actor/errors";
import type { Encoding } from "../../src/actor/protocol/serde";
import { createActorRouter } from "../../src/actor/router";
import { routeWebSocket } from "../../src/actor/router-websocket-endpoints";
import {
	HEADER_CONN_PARAMS,
	HEADER_ENCODING,
} from "../../src/common/actor-router-consts";
import { getLogger } from "../../src/common/log";
import { InlineWebSocketAdapter } from "../../src/common/inline-websocket-adapter";
import type { NativeDatabaseProvider, SqliteDatabase } from "../../src/db/config";
import { deconstructError, stringifyError } from "../../src/common/utils";
import * as cbor from "cbor-x";
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
import {
	CURRENT_VERSION as CLIENT_PROTOCOL_CURRENT_VERSION,
	HTTP_RESPONSE_ERROR_VERSIONED,
} from "../../src/schemas/client-protocol/versioned";
import type * as protocol from "../../src/schemas/client-protocol/mod";
import {
	type HttpResponseError as HttpResponseErrorJson,
	HttpResponseErrorSchema,
} from "../../src/schemas/client-protocol-zod/mod";
import { contentTypeForEncoding, serializeWithEncoding } from "../../src/serde";
import { getEnvUniversal } from "../../src/utils";

function logger() {
	return getLogger("dynamic-actor");
}
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
	kvDeleteRange: IsolateReferenceLike;
	kvListPrefix: IsolateReferenceLike;
	kvListRange: IsolateReferenceLike;
	dbExec: IsolateReferenceLike;
	dbQuery: IsolateReferenceLike;
	dbRun: IsolateReferenceLike;
	dbClose: IsolateReferenceLike;
	setAlarm: IsolateReferenceLike;
	clientCall: IsolateReferenceLike;
	rawDatabaseExecute: IsolateReferenceLike;
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
	disconnect?: () => void;
}

interface DynamicConnStateManagerLike {
	hibernatableData?: DynamicHibernatableConnData;
}

interface DynamicActorDriver {
	loadActor(actorId: string): Promise<DynamicActorInstanceLike>;
	getContext(actorId: string): unknown;
	overrideRawDatabaseClient(actorId: string): Promise<{
		exec: <
			TRow extends Record<string, unknown> = Record<string, unknown>,
		>(
			query: string,
			...args: unknown[]
		) => Promise<TRow[]>;
	}>;
	getNativeSqliteConfig(): {
		endpoint: string;
		namespace: string;
		token?: string;
	};
	getNativeDatabaseProvider(): NativeDatabaseProvider;
	kvBatchPut(actorId: string, entries: Array<[Uint8Array, Uint8Array]>): Promise<void>;
	kvBatchGet(actorId: string, keys: Uint8Array[]): Promise<Array<Uint8Array | null>>;
	kvBatchDelete(actorId: string, keys: Uint8Array[]): Promise<void>;
	kvDeleteRange(actorId: string, start: Uint8Array, end: Uint8Array): Promise<void>;
	kvListPrefix(
		actorId: string,
		prefix: Uint8Array,
	): Promise<Array<[Uint8Array, Uint8Array]>>;
	kvListRange(
		actorId: string,
		start: Uint8Array,
		end: Uint8Array,
		options?: {
			reverse?: boolean;
			limit?: number;
		},
	): Promise<Array<[Uint8Array, Uint8Array]>>;
	setAlarm(actor: { id: string }, timestamp: number): Promise<void>;
	startSleep(actorId: string): void;
	ackHibernatableWebSocketMessage(
		gatewayId: ArrayBuffer,
		requestId: ArrayBuffer,
		serverMessageIndex: number,
	): void;
	startDestroy(actorId: string): void;
}

interface DynamicActorDefinitionLike {
	config: unknown;
	instantiate: () => DynamicActorInstanceLike;
}

interface DynamicActorInstanceLike {
	id: string;
	isStopping: boolean;
	connectionManager: {
		prepareAndConnectConn: (
			driver: unknown,
			parameters: unknown,
			request: Request,
			path: string,
			headers: Record<string, string>,
		) => Promise<DynamicConnLike>;
	};
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
	cleanupPersistedConnections?: (reason?: string) => Promise<number>;
	getHibernatingWebSocketMetadata?: () => Array<{
		gatewayId: ArrayBuffer;
		requestId: ArrayBuffer;
		serverMessageIndex: number;
		clientMessageIndex: number;
		path: string;
		headers: Record<string, string>;
	}>;
	conns: Map<string, DynamicConnLike>;
	handleInboundHibernatableWebSocketMessage?: (
		conn: DynamicConnLike | undefined,
		payload: unknown,
		rivetMessageIndex: number | undefined,
	) => void;
	handleRawRequest: (
		conn: DynamicConnLike,
		request: Request,
	) => Promise<Response>;
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

// isolated-vm's built-in text codecs are incomplete for this runtime.
// Provide minimal Buffer-backed implementations for the encodings used by
// RivetKit and wa-sqlite.
class DynamicTextDecoder {
	readonly encoding: string;

	constructor(label = "utf-8") {
		this.encoding = normalizeTextEncoding(label);
	}

	decode(input?: ArrayBuffer | ArrayBufferView): string {
		if (!input) {
			return "";
		}
		if (ArrayBuffer.isView(input)) {
			return Buffer.from(
				input.buffer,
				input.byteOffset,
				input.byteLength,
			).toString(this.encoding);
		}
		return Buffer.from(input).toString(this.encoding);
	}
}

class DynamicTextEncoder {
	readonly encoding = "utf-8";

	encode(input = ""): Uint8Array {
		return Uint8Array.from(Buffer.from(input, "utf8"));
	}
}

function normalizeTextEncoding(label: string): BufferEncoding {
	switch (label.toLowerCase()) {
		case "utf8":
		case "utf-8":
			return "utf8";
		case "utf16le":
		case "utf-16le":
		case "utf16":
		case "utf-16":
			return "utf16le";
		default:
			throw new Error(
				`unsupported text encoding in dynamic runtime: ${label}`,
			);
	}
}

globalObject.TextDecoder = DynamicTextDecoder as unknown;
globalObject.TextEncoder = DynamicTextEncoder as unknown;

const bootstrapConfig = readBootstrapConfig();
const hostBridge = readHostBridge();

let loadedActor: DynamicActorInstanceLike | undefined;
let loadingActorPromise: Promise<void> | undefined;
let runtimeStatePromise: Promise<DynamicRuntimeState> | undefined;
let runtimeStopMode: "sleep" | "destroy" | undefined;
const webSocketSessions = new Map<
	number,
	{
		ws: WebSocket;
		isHibernatable: boolean;
		conn?: DynamicConnLike;
		actor?: DynamicActorInstanceLike;
		clientCloseInitiated: boolean;
		adapter: {
			dispatchClientMessageWithMetadata?: (
				payload: string | Buffer,
				messageIndex?: number,
			) => void;
		};
	}
>();
const CLIENT_ACCESSOR_METHODS = new Set(["get", "getOrCreate", "getForId", "create"]);
const nativeDatabaseCache = new Map<string, SqliteDatabase>();

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
		kvDeleteRange: getRequiredHostRef(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.kvDeleteRange,
		),
		kvListPrefix: getRequiredHostRef(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.kvListPrefix,
		),
		kvListRange: getRequiredHostRef(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.kvListRange,
		),
		dbExec: getRequiredHostRef(DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.dbExec),
		dbQuery: getRequiredHostRef(DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.dbQuery),
		dbRun: getRequiredHostRef(DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.dbRun),
		dbClose: getRequiredHostRef(DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.dbClose),
		setAlarm: getRequiredHostRef(DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.setAlarm),
		clientCall: getRequiredHostRef(DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.clientCall),
		rawDatabaseExecute: getRequiredHostRef(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.rawDatabaseExecute,
		),
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

function createNativeDatabaseBridge(actorIdValue: string): SqliteDatabase {
	return {
		async exec(
			sql: string,
			callback?: (row: unknown[], columns: string[]) => void,
		): Promise<void> {
			const result = await bridgeCall<{
				columns: string[];
				rows: unknown[][];
			}>(hostBridge.dbExec, [actorIdValue, sql]);
			if (!callback) {
				return;
			}
			for (const row of result.rows) {
				callback(row, result.columns);
			}
		},
		async run(
			sql: string,
			params?: unknown[] | Record<string, unknown>,
		): Promise<void> {
			await bridgeCall(hostBridge.dbRun, [actorIdValue, sql, params]);
		},
		async query(
			sql: string,
			params?: unknown[] | Record<string, unknown>,
		): Promise<{ rows: unknown[][]; columns: string[] }> {
			return await bridgeCall(hostBridge.dbQuery, [actorIdValue, sql, params]);
		},
		async close(): Promise<void> {
			try {
				await bridgeCall(hostBridge.dbClose, [actorIdValue]);
			} finally {
				nativeDatabaseCache.delete(actorIdValue);
			}
		},
	};
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
			dynamicHostLog(
				"debug",
				`loadActor actor.start begin actorId=${bootstrapConfig.actorId}`,
			);
			await actor.start(
				actorDriver,
				inlineClient,
				bootstrapConfig.actorId,
				bootstrapConfig.actorName,
				bootstrapConfig.actorKey,
				"unknown",
			);
			dynamicHostLog(
				"debug",
				`loadActor actor.start complete actorId=${bootstrapConfig.actorId}`,
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
				return (...operationArgs: unknown[]) => {
					const input = {
						actorName,
						accessorMethod,
						accessorArgs,
						operation,
						operationArgs,
					} satisfies DynamicClientCallInput;
					if (shouldHandleLocalClientCall(input)) {
						return handleLocalClientCall(input);
					}
					return bridgeCall(hostBridge.clientCall, [input]);
				};
			},
		},
	);
}

function shouldHandleLocalClientCall(input: DynamicClientCallInput): boolean {
	if (input.actorName !== bootstrapConfig.actorName) {
		return false;
	}

	if (input.accessorMethod !== "getForId") {
		return false;
	}

	if (input.accessorArgs[0] !== bootstrapConfig.actorId) {
		return false;
	}

	return input.operation === "send";
}

async function handleLocalClientCall(
	input: DynamicClientCallInput,
): Promise<unknown> {
	if (input.operation !== "send") {
		throw new Error(
			`unsupported local dynamic client operation: ${input.operation}`,
		);
	}

	const [queueName, body, options] = input.operationArgs as [
		string,
		unknown,
		{ wait?: boolean; timeout?: number } | undefined,
	];
	const actor = (await loadActor(bootstrapConfig.actorId)) as any;
	if (!options?.wait) {
		await actor.queueManager.enqueue(queueName, body);
		return undefined;
	}
	return await actor.queueManager.enqueueAndWait(
		queueName,
		body,
		options.timeout,
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
	async overrideRawDatabaseClient(actorIdValue: string) {
		return {
			exec: async <
				TRow extends Record<string, unknown> = Record<string, unknown>,
			>(
				query: string,
				...args: unknown[]
			): Promise<TRow[]> => {
				return await bridgeCall<TRow[]>(hostBridge.rawDatabaseExecute, [
					actorIdValue,
					query,
					args,
				]);
			},
		};
	},
	getNativeSqliteConfig() {
		return {
			endpoint: bootstrapConfig.endpoint,
			namespace: bootstrapConfig.namespace,
			token: bootstrapConfig.token,
		};
	},
	getNativeDatabaseProvider() {
		return {
			open: async (actorIdValue: string) => {
				dynamicHostLog(
					"debug",
					`openRawDatabaseFromEnvoy begin actorId=${actorIdValue}`,
				);
				const nativeWrapper = loadNativeWrapper();
				const handle = await getOrCreateNativeDatabaseEnvoyHandle();
				const database = await nativeWrapper.openRawDatabaseFromEnvoy(
					handle as Parameters<
						typeof nativeWrapper.openRawDatabaseFromEnvoy
					>[0],
					actorIdValue,
				);
				dynamicHostLog(
					"debug",
					`openRawDatabaseFromEnvoy complete actorId=${actorIdValue}`,
				);
				return database;
			},
		};
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
	async kvDeleteRange(
		actorIdValue: string,
		start: Uint8Array,
		end: Uint8Array,
	): Promise<void> {
		await bridgeCall(hostBridge.kvDeleteRange, [
			actorIdValue,
			toArrayBuffer(start),
			toArrayBuffer(end),
		]);
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
	async kvListRange(
		actorIdValue: string,
		start: Uint8Array,
		end: Uint8Array,
		options?: {
			reverse?: boolean;
			limit?: number;
		},
	): Promise<Array<[Uint8Array, Uint8Array]>> {
		const values = await bridgeCall<Array<[ArrayBuffer, ArrayBuffer]>>(
			hostBridge.kvListRange,
			[
				actorIdValue,
				toArrayBuffer(start),
				toArrayBuffer(end),
				options,
			],
		);
		return values.map(([key, value]) => [new Uint8Array(key), new Uint8Array(value)]);
	},
	async setAlarm(actor, timestamp: number): Promise<void> {
		await bridgeCall(hostBridge.setAlarm, [actor.id, timestamp]);
	},
	getNativeDatabaseProvider(): NativeDatabaseProvider {
		return {
			open: async (actorIdValue: string): Promise<SqliteDatabase> => {
				const existing = nativeDatabaseCache.get(actorIdValue);
				if (existing) {
					return existing;
				}
				const database = createNativeDatabaseBridge(actorIdValue);
				nativeDatabaseCache.set(actorIdValue, database);
				return database;
			},
		};
	},
	startSleep(requestActorId: string): void {
		bridgeCallSync(hostBridge.startSleep, [requestActorId]);
	},
	ackHibernatableWebSocketMessage(
		gatewayId: ArrayBuffer,
		requestId: ArrayBuffer,
		serverMessageIndex: number,
	): void {
		bridgeCallSync(hostBridge.ackHibernatableWebSocketMessage, [
			toArrayBuffer(gatewayId as ArrayBuffer | Uint8Array),
			toArrayBuffer(requestId as ArrayBuffer | Uint8Array),
			serverMessageIndex,
		]);
	},
	startDestroy(requestActorId: string): void {
		bridgeCallSync(hostBridge.startDestroy, [requestActorId]);
	},
};

function patchRequestBodyReaders(
	request: Request,
	requestBody: ArrayBuffer | undefined,
): void {
	if (requestBody === undefined) {
		return;
	}

	const fallbackBody = requestBody.slice(0);
	const fallbackBytes = Buffer.from(fallbackBody);
	const fallbackText = fallbackBytes.toString("utf8");
	Object.defineProperty(request, "arrayBuffer", {
		configurable: true,
		value: async () => fallbackBody.slice(0),
	});
	Object.defineProperty(request, "text", {
		configurable: true,
		value: async () => fallbackText,
	});
	Object.defineProperty(request, "json", {
		configurable: true,
		value: async () => JSON.parse(fallbackText),
	});
}

function decodeRequestBody(bodyBase64?: string | null): Uint8Array | undefined {
	if (!bodyBase64) {
		return undefined;
	}

	return Buffer.from(bodyBase64, "base64");
}

function toExactArrayBuffer(body: Uint8Array | undefined): ArrayBuffer | undefined {
	if (!body) {
		return undefined;
	}

	return body.buffer.slice(
		body.byteOffset,
		body.byteOffset + body.byteLength,
	);
}

function parseRequestConnParams(request: Request): unknown {
	const paramsParam = request.headers.get(HEADER_CONN_PARAMS);
	if (!paramsParam) {
		return null;
	}

	try {
		return JSON.parse(paramsParam);
	} catch (error) {
		throw new errors.InvalidParams(
			`Invalid params JSON: ${stringifyError(error)}`,
		);
	}
}

function getRequestExposeInternalError(): boolean {
	return getEnvUniversal("RIVET_EXPOSE_ERRORS") === "1";
}

function getRequestEncoding(request: Request): Encoding {
	const encodingParam = request.headers.get(HEADER_ENCODING);
	if (!encodingParam) {
		return "json";
	}

	switch (encodingParam) {
		case "json":
		case "cbor":
		case "bare":
			return encodingParam;
		default:
			throw new errors.InvalidEncoding(encodingParam);
	}
}

let nativeDatabaseEnvoyHandlePromise: Promise<unknown> | undefined;

function ensureProcessReportHeader() {
	const report = process.report as
		| {
				getReport?: () => { header?: Record<string, unknown> };
		  }
		| undefined;
	if (!report || typeof report.getReport !== "function") {
		return;
	}

	const originalGetReport = report.getReport.bind(report);
	try {
		const current = originalGetReport();
		if (current?.header) {
			return;
		}
	} catch {
		// Fall through and install the compatibility wrapper below.
	}

	report.getReport = () => {
		const current = originalGetReport();
		return {
			...current,
			header: current?.header ?? {
				glibcVersionRuntime: "2.31",
			},
		};
	};
}

function loadNativeWrapper() {
	ensureProcessReportHeader();
	const specifier = ["@rivetkit", "rivetkit-native", "wrapper"].join("/");
	return require(specifier) as typeof import("@rivetkit/rivetkit-native/wrapper");
}

async function getOrCreateNativeDatabaseEnvoyHandle(): Promise<unknown> {
	if (nativeDatabaseEnvoyHandlePromise) {
		return await nativeDatabaseEnvoyHandlePromise;
	}

	nativeDatabaseEnvoyHandlePromise = (async () => {
		const nativeWrapper = loadNativeWrapper();
		const handle = nativeWrapper.startEnvoySync({
			endpoint: bootstrapConfig.endpoint,
			token: bootstrapConfig.token,
			namespace: bootstrapConfig.namespace,
			poolName: `rivetkit-dynamic-native-db-${process.pid}`,
			version: nativeWrapper.protocol.VERSION,
			prepopulateActorNames: {},
			fetch: async () => new Response(null, { status: 500 }),
			websocket: async () => {},
			hibernatableWebSocket: {
				canHibernate: () => false,
			},
			onActorStart: async () => {},
			onActorStop: async () => {},
			onShutdown: () => {},
		});
		await handle.started();
		return handle;
	})().catch((error) => {
		nativeDatabaseEnvoyHandlePromise = undefined;
		throw error;
	});

	return await nativeDatabaseEnvoyHandlePromise;
}

function buildErrorResponse(request: Request, error: unknown): Response {
	const { statusCode, group, code, message, metadata } = deconstructError(
		error,
		logger(),
		{
			method: request.method,
			path: new URL(request.url).pathname,
		},
		getRequestExposeInternalError(),
	);

	let encoding: Encoding;
	try {
		encoding = getRequestEncoding(request);
	} catch {
		encoding = "json";
	}

	const output = serializeWithEncoding(
		encoding,
		{ group, code, message, metadata },
		HTTP_RESPONSE_ERROR_VERSIONED,
		CLIENT_PROTOCOL_CURRENT_VERSION,
		HttpResponseErrorSchema,
		(value): HttpResponseErrorJson => ({
			group: value.group,
			code: value.code,
			message: value.message,
			metadata: value.metadata,
		}),
		(value): protocol.HttpResponseError => ({
			group: value.group,
			code: value.code,
			message: value.message,
			metadata: value.metadata
				? toExactArrayBuffer(cbor.encode(value.metadata))
				: null,
		}),
	);

	// biome-ignore lint/suspicious/noExplicitAny: serializeWithEncoding returns string | Uint8Array, both valid for Response
	return new Response(output as any, {
		status: statusCode,
		headers: {
			"Content-Type": contentTypeForEncoding(encoding),
		},
	});
}

async function handleDynamicRawRequest(request: Request): Promise<Response> {
	const actor = await loadActor(bootstrapConfig.actorId);
	const requestUrl = new URL(request.url);
	const requestPath = requestUrl.pathname;
	const originalPath = requestPath.replace(/^\/request/, "") || "/";
	const correctedUrl = new URL(
		originalPath + requestUrl.search,
		requestUrl.origin,
	);
	const requestBody =
		request.method !== "GET" &&
		request.method !== "HEAD" &&
		request.body !== null
			? new Uint8Array(await request.arrayBuffer())
			: undefined;
	const correctedRequest = new Request(correctedUrl, {
		method: request.method,
		headers: request.headers,
		body: requestBody,
		duplex: "half",
	} as RequestInit);
	patchRequestBodyReaders(correctedRequest, toExactArrayBuffer(requestBody));
	Object.defineProperty(correctedRequest, "url", {
		configurable: true,
		value: correctedUrl.toString(),
	});

	const headerRecord: Record<string, string> = {};
	request.headers.forEach((value, key) => {
		headerRecord[key] = value;
	});

	let conn: DynamicConnLike | undefined;
	try {
		conn = await actor.connectionManager.prepareAndConnectConn(
			createRawRequestDriver(),
			parseRequestConnParams(request),
			correctedRequest,
			requestPath,
			headerRecord,
		);
		return await actor.handleRawRequest(conn, correctedRequest);
	} finally {
		conn?.disconnect?.();
	}
}

async function dynamicFetchEnvelope(
	url: string,
	method: string,
	headers: Record<string, string>,
	bodyBase64?: string | null,
): Promise<FetchEnvelopeOutput> {
	const requestBody = decodeRequestBody(bodyBase64);
	const request = new Request(url, {
		method,
		headers,
		body: requestBody,
	});
	patchRequestBodyReaders(request, toExactArrayBuffer(requestBody));
	const requestUrl = new URL(request.url);
	let response: Response;
	try {
		response = requestUrl.pathname.startsWith("/request/")
			? await handleDynamicRawRequest(request)
			: await (await getRuntimeState()).actorRouter.fetch(request, {
					actorId: bootstrapConfig.actorId,
				});
	} catch (error) {
		response = buildErrorResponse(request, error);
	}
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
	dynamicHostLog("debug", `dynamic stop mode=${mode}`);
	runtimeStopMode = mode;
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
	const shouldPreserveRawHibernatableConn =
		Boolean(handler.onRestore) && Boolean(handler.conn?.isHibernatable);
	const wrappedHandler = shouldPreserveRawHibernatableConn
		? {
				...handler,
				onClose: (event: any, wsContext: any) => {
					const session = webSocketSessions.get(input.sessionId);
					if (!session?.clientCloseInitiated) {
						return;
					}
					handler.onClose(event, wsContext);
				},
		  }
		: handler;
	// Restored hibernatable sockets must go through the router's onRestore
	// path so the existing persisted connection is rebound instead of being
	// treated like a brand new websocket.
	const adapter = new InlineWebSocketAdapter(wrappedHandler, {
		restoring: Boolean(input.isRestoringHibernatable),
	});
	const ws = adapter.clientWebSocket;
	webSocketSessions.set(input.sessionId, {
		ws,
		isHibernatable: Boolean(input.isHibernatable),
		conn: handler.conn,
		actor: handler.actor as DynamicActorInstanceLike | undefined,
		clientCloseInitiated: false,
		adapter,
	});

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
		dynamicHostLog(
			"warn",
			`dynamic websocket send missing session=${input.sessionId} known=${Array.from(webSocketSessions.keys()).join(",")}`,
		);
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
		// Dynamic actors share the same runtime-owned hibernatable websocket
		// bookkeeping as static actors, but execute it inside the isolate because
		// that is where the actor instance and state manager live.
		session.actor?.handleInboundHibernatableWebSocketMessage?.(
			session.conn,
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
	session.clientCloseInitiated = true;
	session.ws.close(input.code, input.reason);
	return true;
}

async function dynamicGetHibernatingWebSocketsEnvelope(): Promise<
	Array<DynamicHibernatingWebSocketMetadata>
> {
	const actor = await loadActor(bootstrapConfig.actorId);
	if (typeof actor.getHibernatingWebSocketMetadata === "function") {
		return actor.getHibernatingWebSocketMetadata().map((entry) => ({
			gatewayId: toArrayBuffer(entry.gatewayId.slice(0)),
			requestId: toArrayBuffer(entry.requestId.slice(0)),
			serverMessageIndex: entry.serverMessageIndex,
			clientMessageIndex: entry.clientMessageIndex,
			path: entry.path,
			headers: { ...entry.headers },
		}));
	}
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
				gatewayId: toArrayBuffer(hibernatable.gatewayId.slice(0)),
				requestId: toArrayBuffer(hibernatable.requestId.slice(0)),
				serverMessageIndex: hibernatable.serverMessageIndex,
				clientMessageIndex: hibernatable.clientMessageIndex,
				path: hibernatable.requestPath,
				headers: { ...hibernatable.requestHeaders },
			};
		})
		.filter((entry): entry is DynamicHibernatingWebSocketMetadata => {
			return entry !== undefined;
		});
}

async function dynamicCleanupPersistedConnectionsEnvelope(
	reason?: string,
): Promise<number> {
	const actor = await loadActor(bootstrapConfig.actorId);
	return await actor.cleanupPersistedConnections(reason);
}

async function dynamicEnsureStartedEnvelope(): Promise<boolean> {
	await loadActor(bootstrapConfig.actorId);
	return true;
}

async function dynamicDisposeEnvelope(): Promise<boolean> {
	for (const session of webSocketSessions.values()) {
		if (runtimeStopMode === "sleep" && session.isHibernatable) {
			continue;
		}
		try {
			session.ws.close(1001, "dynamic.runtime.disposed");
		} catch {
			// noop
		}
	}
	webSocketSessions.clear();
	runtimeStopMode = undefined;
	for (const [actorId, database] of nativeDatabaseCache.entries()) {
		try {
			await database.close();
		} catch {
			// noop
		}
		nativeDatabaseCache.delete(actorId);
	}
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
	dynamicCleanupPersistedConnectionsEnvelope,
	dynamicEnsureStartedEnvelope,
	dynamicDisposeEnvelope,
};

module.exports = bootstrapExports;
