import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { mkdir, readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import * as cbor from "cbor-x";
import { VirtualWebSocket } from "@rivetkit/virtual-websocket";
import * as errors from "@/actor/errors";
import type { ActorDriver } from "@/actor/driver";
import type { ActorKey } from "@/actor/mod";
import type { Encoding } from "@/actor/protocol/serde";
import {
	HEADER_CONN_PARAMS,
	HEADER_ENCODING,
} from "@/common/actor-router-consts";
import { getLogger } from "@/common/log";
import { deconstructError, stringifyError } from "@/common/utils";
import type { UniversalWebSocket } from "@/common/websocket-interface";
import type { AnyClient } from "@/client/client";
import type { RegistryConfig } from "@/registry/config";
import type * as protocol from "@/schemas/client-protocol/mod";
import {
	CURRENT_VERSION as CLIENT_PROTOCOL_CURRENT_VERSION,
	HTTP_RESPONSE_ERROR_VERSIONED,
} from "@/schemas/client-protocol/versioned";
import {
	type HttpResponseError as HttpResponseErrorJson,
	HttpResponseErrorSchema,
} from "@/schemas/client-protocol-zod/mod";
import { contentTypeForEncoding, serializeWithEncoding } from "@/serde";
import { bufferToArrayBuffer, getEnvUniversal } from "@/utils";
import {
	DYNAMIC_BOOTSTRAP_CONFIG_GLOBAL_KEY,
	DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS,
	DYNAMIC_ISOLATE_EXPORT_GLOBAL_KEYS,
	type DynamicClientCallInput,
	type DynamicHibernatingWebSocketMetadata,
	type DynamicBootstrapExportName,
	type DynamicSourceFormat,
	type FetchEnvelopeInput,
	type FetchEnvelopeOutput,
	type IsolateDispatchPayload,
	type WebSocketCloseEnvelopeInput,
	type WebSocketOpenEnvelopeInput,
	type WebSocketSendEnvelopeInput,
} from "./runtime-bridge";
import {
	createDynamicActorAuthContext,
	createDynamicActorLoaderContext,
	type DynamicActorAuth,
	type DynamicActorLoader,
	type DynamicActorLoadResult,
} from "./internal";
import { compileActorSource } from "./compile";

export type { DynamicHibernatingWebSocketMetadata } from "./runtime-bridge";

const DYNAMIC_SANDBOX_APP_ROOT = "/root";
const DYNAMIC_SANDBOX_TMP_ROOT = "/tmp";
const DYNAMIC_SANDBOX_BOOTSTRAP_FILE = `${DYNAMIC_SANDBOX_APP_ROOT}/dynamic-bootstrap.cjs`;
const DYNAMIC_SANDBOX_HOST_INIT_FILE = `${DYNAMIC_SANDBOX_APP_ROOT}/dynamic-host-init.cjs`;
const DYNAMIC_SANDBOX_BOOTSTRAP_ENTRY_FILE = `${DYNAMIC_SANDBOX_APP_ROOT}/dynamic-bootstrap-entry.cjs`;

let dynamicRuntimeModuleAccessCwdPromise: Promise<string> | undefined;
let secureExecModulePromise: Promise<SecureExecModule> | undefined;
let isolatedVmModulePromise: Promise<IsolatedVmModule> | undefined;

function logger() {
	return getLogger("dynamic-actor");
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

function getRequestConnParams(request: Request): unknown {
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
	return (
		getEnvUniversal("RIVET_EXPOSE_ERRORS") === "1" ||
		getEnvUniversal("NODE_ENV") === "development"
	);
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
				? bufferToArrayBuffer(cbor.encode(value.metadata))
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

function normalizeRequestUrl(pathValue: string): string {
	if (pathValue.startsWith("http://") || pathValue.startsWith("https://")) {
		return pathValue;
	}
	return pathValue.startsWith("/")
		? `http://actor${pathValue}`
		: `http://actor/${pathValue}`;
}

interface SecureExecModule {
	NodeProcess?: new (options: Record<string, unknown>) => NodeProcessLike;
	NodeRuntime?: new (options: Record<string, unknown>) => NodeProcessLike;
	createInMemoryFileSystem: () => InMemoryFileSystemLike;
	createNodeDriver?: (options: Record<string, unknown>) => unknown;
	createNodeRuntimeDriverFactory?: () => unknown;
}

interface IsolatedVmModule {
	Reference: new <T>(value: T) => ReferenceLike<T>;
	ExternalCopy: new <T>(
		value: T,
	) => {
		copy(): T;
	};
}

interface SecureExecFsAccessRequest {
	op:
		| "read"
		| "write"
		| "mkdir"
		| "createDir"
		| "readdir"
		| "stat"
		| "rm"
		| "rename"
		| "exists";
	path: string;
}

interface SecureExecNetworkAccessRequest {
	op: "fetch" | "http" | "dns" | "listen";
	url?: string;
	method?: string;
	hostname?: string;
}

interface ReferenceLike<T> {
	apply(
		receiver: unknown,
		args: unknown[],
		options?: Record<string, unknown>,
	): unknown;
	release?(): void;
}

interface NodeProcessLike {
	__unsafeIsoalte: {
		compileScript(
			code: string,
			options?: Record<string, unknown>,
		): Promise<{ run(context: unknown): Promise<void> }>;
	};
	__unsafeCreateContext(
		options?: Record<string, unknown>,
	): Promise<ContextLike>;
	dispose(): void;
}

interface InMemoryFileSystemLike {
	writeFile(path: string, content: string | Uint8Array): Promise<void>;
}

function createSecureExecNodeProcess(
	secureExec: SecureExecModule,
	options: Record<string, unknown>,
): NodeProcessLike {
	if (secureExec.NodeProcess) {
		return new secureExec.NodeProcess(options);
	}

	if (
		secureExec.NodeRuntime &&
		secureExec.createNodeDriver &&
		secureExec.createNodeRuntimeDriverFactory
	) {
		return new secureExec.NodeRuntime({
			systemDriver: secureExec.createNodeDriver({
				filesystem: options.filesystem,
				moduleAccess: options.moduleAccess,
				permissions: options.permissions,
				processConfig: options.processConfig,
				osConfig: options.osConfig,
			}),
			runtimeDriverFactory: secureExec.createNodeRuntimeDriverFactory(),
			memoryLimit: options.memoryLimit,
			cpuTimeLimitMs: options.cpuTimeLimitMs,
			timingMitigation: options.timingMitigation,
		});
	}

	throw new Error(
		"secure-exec runtime is missing both NodeProcess and NodeRuntime support",
	);
}

interface ContextLike {
	global: {
		set(
			name: string,
			value: unknown,
			options?: Record<string, unknown>,
		): Promise<void>;
		get(
			name: string,
			options?: Record<string, unknown>,
		): Promise<ReferenceLike<unknown>>;
	};
	release(): void;
}

interface HostWebSocketSession {
	id: number;
	readyState: 0 | 1 | 2 | 3;
	websocket: VirtualWebSocket;
	isHibernatable: boolean;
	dispatchReady: boolean;
	pendingDispatches: IsolateDispatchPayload[];
	pendingMessages: Array<
		Extract<IsolateDispatchPayload, { type: "message" }>
	>;
}

interface DynamicActorIsolateRuntimeConfig {
	actorId: string;
	actorName: string;
	actorKey: ActorKey;
	input: unknown;
	region: string;
	loader: DynamicActorLoader;
	auth?: DynamicActorAuth;
	actorDriver: ActorDriver;
	inlineClient: AnyClient;
}

interface DynamicRuntimeRefs {
	fetch: ReferenceLike<
		(input: FetchEnvelopeInput) => Promise<FetchEnvelopeOutput>
	>;
	dispatchAlarm: ReferenceLike<() => Promise<boolean>>;
	stop: ReferenceLike<(mode: "sleep" | "destroy") => Promise<boolean>>;
	openWebSocket: ReferenceLike<
		(input: WebSocketOpenEnvelopeInput) => Promise<boolean>
	>;
	sendWebSocket: ReferenceLike<
		(input: WebSocketSendEnvelopeInput) => Promise<boolean>
	>;
	closeWebSocket: ReferenceLike<
		(input: WebSocketCloseEnvelopeInput) => Promise<boolean>
	>;
	getHibernatingWebSockets: ReferenceLike<
		() => Promise<Array<DynamicHibernatingWebSocketMetadata>>
	>;
	cleanupPersistedConnections: ReferenceLike<
		(reason?: string) => Promise<number>
	>;
	ensureStarted: ReferenceLike<() => Promise<boolean>>;
	dispose: ReferenceLike<() => Promise<boolean>>;
}

interface NormalizedDynamicActorLoadResult extends DynamicActorLoadResult {
	sourceFormat: DynamicSourceFormat;
}

interface MaterializedDynamicSource {
	sourcePath: string;
	sourceCode: string;
	sourceEntry: string;
	sourceFormat: DynamicSourceFormat;
}

export interface DynamicWebSocketOpenOptions {
	headers?: Record<string, string>;
	gatewayId?: ArrayBuffer;
	requestId?: ArrayBuffer;
	isHibernatable?: boolean;
	isRestoringHibernatable?: boolean;
}

/**
 * Manages one long lived dynamic actor isolate for a single actor id.
 *
 * The host owns isolate creation, bridge wiring, request forwarding, websocket
 * session mapping, and final disposal. The isolate lifetime matches actor
 * lifetime and survives across fetch and websocket calls until sleep or destroy.
 */
export class DynamicActorIsolateRuntime {
	#config: DynamicActorIsolateRuntimeConfig;

	#nodeProcess: NodeProcessLike | undefined;
	#context: ContextLike | undefined;
	#refs: DynamicRuntimeRefs | undefined;

	#referenceHandles: Array<{ release?: () => void }> = [];
	#webSocketSessions = new Map<number, HostWebSocketSession>();
	#sessionIdsByWebSocket = new WeakMap<UniversalWebSocket, number>();
	#nextWebSocketSessionId = 1;
	#started = false;
	#disposed = false;
	#stopMode: "sleep" | "destroy" | undefined;

	constructor(config: DynamicActorIsolateRuntimeConfig) {
		this.#config = config;
	}

	get #runtimeRefs(): DynamicRuntimeRefs {
		if (!this.#refs) {
			throw new Error("dynamic runtime refs are not initialized");
		}
		return this.#refs;
	}

	async start(): Promise<void> {
		if (this.#started) return;
		if (this.#disposed) {
			throw new Error("dynamic runtime has been disposed");
		}

		logger().debug({
			msg: "dynamic runtime start begin",
			actorId: this.#config.actorId,
		});
		const moduleAccessCwd = await resolveDynamicRuntimeModuleAccessCwd();
		logger().debug({
			msg: "dynamic runtime module access ready",
			actorId: this.#config.actorId,
			moduleAccessCwd,
		});

		const loadResult = await this.#config.loader(
			createDynamicActorLoaderContext(
				this.#config.inlineClient,
				this.#config.actorId,
				this.#config.actorName,
				this.#config.actorKey,
				this.#config.input,
				this.#config.region,
			),
		);
		const normalizedLoadResult = normalizeLoadResult(loadResult);
		logger().debug({
			msg: "dynamic runtime loader resolved source",
			actorId: this.#config.actorId,
		});

		const materializedSource =
			await materializeDynamicSource(normalizedLoadResult);
		logger().debug({
			msg: "dynamic runtime source written",
			actorId: this.#config.actorId,
			sourcePath: materializedSource.sourcePath,
			sourceEntry: materializedSource.sourceEntry,
			sourceFormat: materializedSource.sourceFormat,
		});

		const bootstrapSourcePath =
			await resolveDynamicIsolateRuntimeBootstrapEntryPath();
		const bootstrapSource = await readFile(bootstrapSourcePath, "utf8");
		logger().debug({
			msg: "dynamic runtime bootstrap written",
			actorId: this.#config.actorId,
			bootstrapSourcePath,
			bootstrapPath: DYNAMIC_SANDBOX_BOOTSTRAP_FILE,
		});

		const secureExec = await loadSecureExecModule();
		const ivm = await loadIsolatedVmModule();
		const sandboxFileSystem = secureExec.createInMemoryFileSystem();
		await sandboxFileSystem.writeFile(
			path.posix.join(
				DYNAMIC_SANDBOX_APP_ROOT,
				materializedSource.sourceEntry,
			),
			materializedSource.sourceCode,
		);
		await sandboxFileSystem.writeFile(
			DYNAMIC_SANDBOX_BOOTSTRAP_FILE,
			bootstrapSource,
		);

		const permissions = buildLockedDownPermissions();

		this.#nodeProcess = createSecureExecNodeProcess(secureExec, {
			filesystem: sandboxFileSystem,
			moduleAccess: {
				cwd: moduleAccessCwd,
			},
			// Dynamic actors rely on wall-clock time for schedule.after(),
			// sleep timers, and other persisted actor semantics.
			timingMitigation: "off",
			permissions,
			processConfig: {
				cwd: DYNAMIC_SANDBOX_APP_ROOT,
				env: {
					HOME: DYNAMIC_SANDBOX_APP_ROOT,
					XDG_DATA_HOME: `${DYNAMIC_SANDBOX_APP_ROOT}/.local/share`,
					XDG_CACHE_HOME: `${DYNAMIC_SANDBOX_APP_ROOT}/.cache`,
					TMPDIR: DYNAMIC_SANDBOX_TMP_ROOT,
					RIVET_EXPOSE_ERRORS: "1",
				},
			},
			osConfig: {
				homedir: DYNAMIC_SANDBOX_APP_ROOT,
				tmpdir: DYNAMIC_SANDBOX_TMP_ROOT,
			},
			memoryLimit: normalizedLoadResult.nodeProcess?.memoryLimit,
			cpuTimeLimitMs: normalizedLoadResult.nodeProcess?.cpuTimeLimitMs,
		});

		this.#context = await this.#nodeProcess.__unsafeCreateContext({
			cwd: DYNAMIC_SANDBOX_APP_ROOT,
			filePath: DYNAMIC_SANDBOX_HOST_INIT_FILE,
		});
		logger().debug({
			msg: "dynamic runtime isolate context created",
			actorId: this.#config.actorId,
		});

		await this.#setIsolateBridge(ivm, materializedSource);
		logger().debug({
			msg: "dynamic runtime isolate bridge set",
			actorId: this.#config.actorId,
		});
		await this.#loadBootstrap(DYNAMIC_SANDBOX_BOOTSTRAP_FILE);
		logger().debug({
			msg: "dynamic runtime bootstrap loaded",
			actorId: this.#config.actorId,
		});
		await this.#captureIsolateExports();
		logger().debug({
			msg: "dynamic runtime isolate exports captured",
			actorId: this.#config.actorId,
		});

		this.#started = true;
		logger().debug({
			msg: "dynamic runtime start complete",
			actorId: this.#config.actorId,
		});
	}

	async fetch(request: Request): Promise<Response> {
		try {
			await this.#authorizeRequest(
				request,
				getRequestConnParams(request),
			);
		} catch (error) {
			return buildErrorResponse(request, error);
		}

		const refs = this.#runtimeRefs;
		const input = await requestToEnvelope(request);
		const envelope = (await refs.fetch.apply(undefined, [input], {
			arguments: {
				copy: true,
			},
			result: {
				copy: true,
				promise: true,
			},
		})) as FetchEnvelopeOutput;
		return envelopeToResponse(envelope);
	}

	async openWebSocket(
		pathValue: string,
		encoding: Encoding,
		params: unknown,
		options: DynamicWebSocketOpenOptions = {},
	): Promise<UniversalWebSocket> {
		const request = new Request(normalizeRequestUrl(pathValue), {
			method: "GET",
			headers: options.headers,
		});
		await this.#authorizeRequest(request, params);

		const refs = this.#runtimeRefs;

		const sessionId = this.#nextWebSocketSessionId;
		this.#nextWebSocketSessionId += 1;

		const session: HostWebSocketSession = {
			id: sessionId,
			readyState: 0,
			websocket: new VirtualWebSocket({
				getReadyState: () => session.readyState,
				onSend: (data) => {
					void this.#sendWebSocketMessage(session.id, data);
				},
				onClose: (code, reason) => {
					session.readyState = 2;
					// Runtime disposal can synchronously close host sockets after the
					// isolate bridge has already been torn down. This close callback is
					// cleanup-only in that state, so skip the isolate round trip.
					if (this.#disposed || !this.#refs) {
						return;
					}
					void this.#closeWebSocketMessage(session.id, code, reason);
				},
			}),
			isHibernatable: Boolean(options.isHibernatable),
			dispatchReady: false,
			pendingDispatches: [],
			pendingMessages: [],
		};
		setIndexedWebSocketTestSender(
			session.websocket,
			(data, rivetMessageIndex) =>
				this.#sendWebSocketMessage(session.id, data, rivetMessageIndex),
			this.#config.runtimeConfig.testEnabled,
		);
		this.#webSocketSessions.set(session.id, session);
		this.#sessionIdsByWebSocket.set(session.websocket, session.id);

		try {
			await refs.openWebSocket.apply(
				undefined,
				[
					{
						sessionId,
						path: pathValue,
						encoding,
						params,
						headers: options.headers,
						gatewayId: options.gatewayId,
						requestId: options.requestId,
						isHibernatable: options.isHibernatable,
						isRestoringHibernatable:
							options.isRestoringHibernatable,
					} satisfies WebSocketOpenEnvelopeInput,
				],
				{
					arguments: {
						copy: true,
					},
					result: {
						copy: true,
						promise: true,
					},
				},
			);
		} catch (error) {
			this.#webSocketSessions.delete(session.id);
			session.readyState = 3;
			session.websocket.triggerError(error);
			session.websocket.triggerClose(
				1011,
				"dynamic.websocket.open_failed",
				false,
			);
			throw error;
		}

		session.dispatchReady = true;
		setTimeout(() => {
			this.#flushPendingWebSocketDispatches(session.id);
		}, 0);

		return session.websocket;
	}

	async dispatchAlarm(): Promise<void> {
		const refs = this.#runtimeRefs;
		await refs.dispatchAlarm.apply(undefined, [], {
			result: {
				copy: true,
				promise: true,
			},
		});
	}

	async getHibernatingWebSockets(): Promise<
		Array<DynamicHibernatingWebSocketMetadata>
	> {
		const refs = this.#runtimeRefs;
		const entries = await refs.getHibernatingWebSockets.apply(
			undefined,
			[],
			{
				result: {
					copy: true,
					promise: true,
				},
			},
		);
		return entries as Array<DynamicHibernatingWebSocketMetadata>;
	}

	async cleanupPersistedConnections(reason?: string): Promise<number> {
		const refs = this.#runtimeRefs;
		const count = await refs.cleanupPersistedConnections.apply(
			undefined,
			[reason],
			{
				arguments: {
					copy: true,
				},
				result: {
					copy: true,
					promise: true,
				},
			},
		);
		return count as number;
	}

	async forwardIncomingWebSocketMessage(
		websocket: UniversalWebSocket,
		data: string | ArrayBufferLike | Blob | ArrayBufferView,
		rivetMessageIndex?: number,
	): Promise<void> {
		const sessionId = this.#sessionIdsByWebSocket.get(websocket);
		if (!sessionId) {
			throw new Error("dynamic runtime websocket session not found");
		}
		await this.#sendWebSocketMessage(sessionId, data, rivetMessageIndex);
	}

	async stop(mode: "sleep" | "destroy"): Promise<void> {
		this.#stopMode = mode;
		const refs = this.#runtimeRefs;
		await refs.stop.apply(undefined, [mode], {
			arguments: {
				copy: true,
			},
			result: {
				copy: true,
				promise: true,
			},
		});
	}

	async dispose(): Promise<void> {
		if (this.#disposed) return;
		this.#disposed = true;

		for (const session of this.#webSocketSessions.values()) {
			if (this.#stopMode === "sleep" && session.isHibernatable) {
				continue;
			}
			session.readyState = 3;
			session.websocket.triggerClose(
				1001,
				"dynamic.runtime.disposed",
				false,
			);
		}
		this.#webSocketSessions.clear();
		this.#sessionIdsByWebSocket = new WeakMap<UniversalWebSocket, number>();

		if (this.#refs && this.#stopMode !== "sleep") {
			try {
				await this.#refs.dispose.apply(undefined, [], {
					result: {
						copy: true,
						promise: true,
					},
				});
			} catch (error) {
				logger().warn({
					msg: "failed to dispose isolate runtime state",
					actorId: this.#config.actorId,
					error: stringifyError(error),
				});
			}
		}

		for (const handle of this.#referenceHandles) {
			try {
				handle.release?.();
			} catch {}
		}
		this.#referenceHandles = [];

		try {
			this.#context?.release();
		} catch {}
		this.#context = undefined;

		try {
			this.#nodeProcess?.dispose();
		} catch {}
		this.#nodeProcess = undefined;

		this.#refs = undefined;
		this.#started = false;
		this.#stopMode = undefined;
	}

	async #sendWebSocketMessage(
		sessionId: number,
		data: string | ArrayBufferLike | Blob | ArrayBufferView,
		rivetMessageIndex?: number,
	): Promise<void> {
		const refs = this.#runtimeRefs;

		if (typeof data === "string") {
			await refs.sendWebSocket.apply(
				undefined,
				[
					{
						sessionId,
						kind: "text",
						text: data,
						rivetMessageIndex,
					} satisfies WebSocketSendEnvelopeInput,
				],
				{
					arguments: { copy: true },
					result: { copy: true, promise: true },
				},
			);
			return;
		}

		const binary = await normalizeBinaryPayload(data);
		await refs.sendWebSocket.apply(
			undefined,
			[
				{
					sessionId,
					kind: "binary",
					data: copyUint8ArrayToArrayBuffer(binary),
					rivetMessageIndex,
				} satisfies WebSocketSendEnvelopeInput,
			],
			{
				arguments: { copy: true },
				result: { copy: true, promise: true },
			},
		);
	}

	async #closeWebSocketMessage(
		sessionId: number,
		code: number,
		reason: string,
	): Promise<void> {
		const refs = this.#runtimeRefs;
		await refs.closeWebSocket.apply(
			undefined,
			[
				{
					sessionId,
					code,
					reason,
				} satisfies WebSocketCloseEnvelopeInput,
			],
			{
				arguments: { copy: true },
				result: { copy: true, promise: true },
			},
		);
	}

	async #authorizeRequest(
		request: Request | undefined,
		params: unknown,
	): Promise<void> {
		const auth = this.#config.auth;
		if (!auth) {
			return;
		}

		const context = createDynamicActorAuthContext(
			this.#config.inlineClient,
			this.#config.actorId,
			this.#config.actorName,
			this.#config.actorKey,
			this.#config.input,
			this.#config.region,
			request,
		);
		await auth(context, params);
	}

	async #setIsolateBridge(
		ivm: IsolatedVmModule,
		source: MaterializedDynamicSource,
	): Promise<void> {
		if (!this.#context) {
			throw new Error("missing isolate context");
		}

		// Wire isolate to host callbacks. Every callback here is required for
		// dynamic actor parity and must fail by default if the driver cannot
		// satisfy it.
		const context = this.#context;
		const makeRef = <T>(value: T): ReferenceLike<T> => {
			const ref = new ivm.Reference(value);
			this.#referenceHandles.push(ref as { release?: () => void });
			return ref;
		};
		const makeExternalCopy = <T>(value: T): { copy(): T } => {
			return new ivm.ExternalCopy(value);
		};

		const kvBatchPutRef = makeRef(
			async (
				actorId: string,
				entries: Array<[ArrayBuffer, ArrayBuffer]>,
			): Promise<void> => {
				const decodedEntries = entries.map(
					([key, value]) =>
						[new Uint8Array(key), new Uint8Array(value)] as [
							Uint8Array,
							Uint8Array,
						],
				);
				await this.#config.actorDriver.kvBatchPut(
					actorId,
					decodedEntries,
				);
			},
		);
		const kvBatchGetRef = makeRef(
			async (
				actorId: string,
				keys: ArrayBuffer[],
			): Promise<{ copy(): Array<ArrayBuffer | null> }> => {
				const decodedKeys = keys.map((key) => new Uint8Array(key));
				const values = await this.#config.actorDriver.kvBatchGet(
					actorId,
					decodedKeys,
				);
				return makeExternalCopy(
					values.map((value) =>
						value ? copyUint8ArrayToArrayBuffer(value) : null,
					),
				);
			},
		);
		const kvBatchDeleteRef = makeRef(
			async (actorId: string, keys: ArrayBuffer[]): Promise<void> => {
				const decodedKeys = keys.map((key) => new Uint8Array(key));
				await this.#config.actorDriver.kvBatchDelete(
					actorId,
					decodedKeys,
				);
			},
		);
		const kvListPrefixRef = makeRef(
			async (
				actorId: string,
				prefix: ArrayBuffer,
			): Promise<{ copy(): Array<[ArrayBuffer, ArrayBuffer]> }> => {
				const decodedPrefix = new Uint8Array(prefix);
				const entries = await this.#config.actorDriver.kvListPrefix(
					actorId,
					decodedPrefix,
				);
				return makeExternalCopy(
					entries.map(([key, value]) => [
						copyUint8ArrayToArrayBuffer(key),
						copyUint8ArrayToArrayBuffer(value),
					]),
				);
			},
		);
		const setAlarmRef = makeRef(
			async (actorId: string, timestamp: number): Promise<void> => {
				await this.#config.actorDriver.setAlarm(
					{ id: actorId } as never,
					timestamp,
				);
			},
		);
		const clientCallRef = makeRef(
			async (
				input: DynamicClientCallInput,
			): Promise<{ copy(): unknown }> => {
				const accessor = (
					this.#config.inlineClient as Record<string, any>
				)[input.actorName];
				if (!accessor) {
					throw new Error(
						`dynamic client actor accessor not found: ${input.actorName}`,
					);
				}

				const accessorFn = accessor[input.accessorMethod];
				if (typeof accessorFn !== "function") {
					throw new Error(
						`dynamic client accessor method not found: ${input.actorName}.${input.accessorMethod}`,
					);
				}

				let handle = accessorFn.apply(
					accessor,
					input.accessorArgs ?? [],
				);
				if (handle && typeof handle.then === "function") {
					handle = await handle;
				}

				const operationFn = handle?.[input.operation];
				if (typeof operationFn !== "function") {
					throw new Error(
						`dynamic client operation not found: ${input.actorName}.${input.accessorMethod}(...).${input.operation}`,
					);
				}

				const result = await operationFn.apply(
					handle,
					input.operationArgs ?? [],
				);
				return makeExternalCopy(result);
			},
		);
		const ackHibernatableWebSocketMessageRef = makeRef(
			(
				gatewayId: ArrayBuffer,
				requestId: ArrayBuffer,
				serverMessageIndex: number,
			): void => {
				if (
					typeof this.#config.actorDriver
						.ackHibernatableWebSocketMessage !== "function"
				) {
					throw new Error(
						"driver does not implement ackHibernatableWebSocketMessage",
					);
				}
				this.#config.actorDriver.ackHibernatableWebSocketMessage(
					gatewayId,
					requestId,
					serverMessageIndex,
				);
			},
		);
		const startSleepRef = makeRef((actorId: string): void => {
			if (typeof this.#config.actorDriver.startSleep !== "function") {
				throw new Error("driver does not implement startSleep");
			}
			this.#config.actorDriver.startSleep(actorId);
		});
		const startDestroyRef = makeRef((actorId: string): void => {
			void this.#config.actorDriver.startDestroy(actorId);
		});
		const dispatchRef = makeRef((payload: IsolateDispatchPayload): void => {
			this.#handleIsolateDispatch(payload);
		});
		const logRef = makeRef(
			(level: "debug" | "warn", message: string): void => {
				if (level === "debug") {
					logger().debug({
						msg: "dynamic isolate",
						actorId: this.#config.actorId,
						message,
					});
					return;
				}
				logger().warn({
					msg: "dynamic isolate",
					actorId: this.#config.actorId,
					message,
				});
			},
		);
		const sqliteExecRef = makeRef(
			(
				actorId: string,
				sql: string,
				params: string,
			): { copy(): string } => {
				if (
					typeof this.#config.actorDriver.sqliteExec !== "function"
				) {
					throw new Error(
						"driver does not implement sqliteExec",
					);
				}
				const parsedParams = JSON.parse(params) as unknown[];
				const result = this.#config.actorDriver.sqliteExec(
					actorId,
					sql,
					parsedParams,
				);
				return makeExternalCopy(JSON.stringify(result));
			},
		);
		const sqliteBatchRef = makeRef(
			(
				actorId: string,
				statementsJson: string,
			): { copy(): string } => {
				if (
					typeof this.#config.actorDriver.sqliteBatch !==
					"function"
				) {
					throw new Error(
						"driver does not implement sqliteBatch",
					);
				}
				const statements = JSON.parse(statementsJson) as {
					sql: string;
					params: unknown[];
				}[];
				const results = this.#config.actorDriver.sqliteBatch(
					actorId,
					statements,
				);
				return makeExternalCopy(JSON.stringify(results));
			},
		);

		await context.global.set(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.kvBatchPut,
			kvBatchPutRef,
		);
		await context.global.set(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.kvBatchGet,
			kvBatchGetRef,
		);
		await context.global.set(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.kvBatchDelete,
			kvBatchDeleteRef,
		);
		await context.global.set(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.kvListPrefix,
			kvListPrefixRef,
		);
		await context.global.set(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.setAlarm,
			setAlarmRef,
		);
		await context.global.set(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.clientCall,
			clientCallRef,
		);
		await context.global.set(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.ackHibernatableWebSocketMessage,
			ackHibernatableWebSocketMessageRef,
		);
		await context.global.set(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.startSleep,
			startSleepRef,
		);
		await context.global.set(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.startDestroy,
			startDestroyRef,
		);
		await context.global.set(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.dispatch,
			dispatchRef,
		);
		await context.global.set(DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.log, logRef);
		await context.global.set(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.sqliteExec,
			sqliteExecRef,
		);
		await context.global.set(
			DYNAMIC_HOST_BRIDGE_GLOBAL_KEYS.sqliteBatch,
			sqliteBatchRef,
		);
		await context.global.set(
			DYNAMIC_BOOTSTRAP_CONFIG_GLOBAL_KEY,
			{
				actorId: this.#config.actorId,
				actorName: this.#config.actorName,
				actorKey: this.#config.actorKey,
				sourceEntry: source.sourceEntry,
				sourceFormat: source.sourceFormat,
			},
			{
				copy: true,
			},
		);
	}

	async #loadBootstrap(bootstrapPath: string): Promise<void> {
		if (!this.#context || !this.#nodeProcess) {
			throw new Error("missing isolate bootstrap dependencies");
		}

		// Execute the isolate bootstrap module and copy each required exported
		// envelope function onto known isolate globals so the host can capture
		// references by stable key names.
		const isolateExportGlobalKeys = JSON.stringify(
			DYNAMIC_ISOLATE_EXPORT_GLOBAL_KEYS,
		);
		logger().debug({
			msg: "dynamic runtime bootstrap compile begin",
			actorId: this.#config.actorId,
			bootstrapPath,
		});
		const bootstrapScript =
			await this.#nodeProcess.__unsafeIsoalte.compileScript(
				`
				const bootstrap = require(${JSON.stringify(bootstrapPath)});
				const isolateExportGlobalKeys = ${isolateExportGlobalKeys};
				for (const [exportName, globalKey] of Object.entries(isolateExportGlobalKeys)) {
					const value = bootstrap[exportName];
					if (typeof value !== "function") {
						throw new Error(\`dynamic bootstrap is missing export: \${exportName}\`);
					}
					globalThis[globalKey] = value;
				}
				`,
				{
					filename: DYNAMIC_SANDBOX_BOOTSTRAP_ENTRY_FILE,
				},
			);
		logger().debug({
			msg: "dynamic runtime bootstrap compile complete",
			actorId: this.#config.actorId,
		});
		logger().debug({
			msg: "dynamic runtime bootstrap run begin",
			actorId: this.#config.actorId,
		});
		await bootstrapScript.run(this.#context);
		logger().debug({
			msg: "dynamic runtime bootstrap run complete",
			actorId: this.#config.actorId,
		});
	}

	async #captureIsolateExports(): Promise<void> {
		if (!this.#context) {
			throw new Error("missing isolate context");
		}

		// Capture all envelope handlers from isolate globals once at startup.
		// Later request and websocket operations call these references directly.
		const getRef = async <T>(name: string): Promise<ReferenceLike<T>> => {
			const ref = (await this.#context!.global.get(name, {
				reference: true,
			})) as ReferenceLike<T>;
			this.#referenceHandles.push(ref as { release?: () => void });
			return ref;
		};

		const getExportRef = async <T>(
			exportName: DynamicBootstrapExportName,
		): Promise<ReferenceLike<T>> => {
			return await getRef<T>(
				DYNAMIC_ISOLATE_EXPORT_GLOBAL_KEYS[exportName],
			);
		};

		this.#refs = {
			fetch: await getExportRef("dynamicFetchEnvelope"),
			dispatchAlarm: await getExportRef("dynamicDispatchAlarmEnvelope"),
			stop: await getExportRef("dynamicStopEnvelope"),
			openWebSocket: await getExportRef("dynamicOpenWebSocketEnvelope"),
			sendWebSocket: await getExportRef("dynamicWebSocketSendEnvelope"),
			closeWebSocket: await getExportRef("dynamicWebSocketCloseEnvelope"),
			getHibernatingWebSockets: await getExportRef(
				"dynamicGetHibernatingWebSocketsEnvelope",
			),
			cleanupPersistedConnections: await getExportRef(
				"dynamicCleanupPersistedConnectionsEnvelope",
			),
			ensureStarted: await getExportRef("dynamicEnsureStartedEnvelope"),
			dispose: await getExportRef("dynamicDisposeEnvelope"),
		};
	}

	#handleIsolateDispatch(payload: IsolateDispatchPayload): void {
		const session = this.#webSocketSessions.get(payload.sessionId);
		if (!session) {
			return;
		}

		if (!session.dispatchReady) {
			session.pendingDispatches.push(payload);
			return;
		}

		this.#dispatchIsolatePayload(session, payload);
	}

	#flushPendingWebSocketDispatches(sessionId: number): void {
		const session = this.#webSocketSessions.get(sessionId);
		if (!session || !session.dispatchReady) {
			return;
		}

		if (session.pendingDispatches.length === 0) {
			return;
		}

		const pendingDispatches = session.pendingDispatches;
		session.pendingDispatches = [];

		for (const payload of pendingDispatches) {
			const currentSession = this.#webSocketSessions.get(sessionId);
			if (!currentSession || !currentSession.dispatchReady) {
				return;
			}
			this.#dispatchIsolatePayload(currentSession, payload);
		}
	}

	#dispatchIsolatePayload(
		session: HostWebSocketSession,
		payload: IsolateDispatchPayload,
	): void {
		switch (payload.type) {
			case "open": {
				session.readyState = 1;
				session.websocket.triggerOpen();
				for (const pendingMessage of session.pendingMessages) {
					this.#dispatchIsolateWebSocketMessage(
						session,
						pendingMessage,
					);
				}
				session.pendingMessages = [];
				break;
			}
			case "message": {
				if (session.readyState !== 1) {
					session.pendingMessages.push(payload);
					break;
				}
				this.#dispatchIsolateWebSocketMessage(session, payload);
				break;
			}
			case "close": {
				session.readyState = 3;
				session.websocket.triggerClose(
					payload.code ?? 1000,
					payload.reason ?? "",
					payload.wasClean,
				);
				this.#webSocketSessions.delete(payload.sessionId);
				break;
			}
			case "error": {
				session.websocket.triggerError(
					new Error(payload.message ?? "dynamic websocket error"),
				);
				break;
			}
		}
	}

	#dispatchIsolateWebSocketMessage(
		session: HostWebSocketSession,
		payload: Extract<IsolateDispatchPayload, { type: "message" }>,
	): void {
		if (payload.kind === "text") {
			(session.websocket as any).triggerMessage(
				payload.text ?? "",
				payload.rivetMessageIndex,
			);
			return;
		}
		const bytes = payload.data
			? Buffer.from(new Uint8Array(payload.data))
			: Buffer.alloc(0);
		(session.websocket as any).triggerMessage(
			bytes,
			payload.rivetMessageIndex,
		);
	}
}

function normalizeLoadResult(
	loadResult: DynamicActorLoadResult,
): NormalizedDynamicActorLoadResult {
	if (!loadResult || typeof loadResult.source !== "string") {
		throw new Error(
			"dynamic actor loader must return an object with a string `source` property",
		);
	}

	const sourceFormat = loadResult.sourceFormat ?? "esm-js";
	if (
		sourceFormat !== "commonjs-js" &&
		sourceFormat !== "esm-js" &&
		sourceFormat !== "commonjs-ts" &&
		sourceFormat !== "esm-ts"
	) {
		throw new Error(
			"dynamic actor loader returned unsupported `sourceFormat`. Expected `commonjs-js`, `esm-js`, `commonjs-ts`, or `esm-ts`.",
		);
	}

	return {
		...loadResult,
		sourceFormat,
	};
}

async function requestToEnvelope(
	request: Request,
): Promise<FetchEnvelopeInput> {
	const headers: Record<string, string> = {};
	request.headers.forEach((value, key) => {
		headers[key] = value;
	});

	let body: ArrayBuffer | undefined;
	if (request.method !== "GET" && request.method !== "HEAD") {
		const requestBody = await request.arrayBuffer();
		if (requestBody.byteLength > 0) {
			body = requestBody.slice(0);
		}
	}

	return {
		url: request.url,
		method: request.method,
		headers,
		body,
	};
}

function envelopeToResponse(envelope: FetchEnvelopeOutput): Response {
	return new Response(new Uint8Array(envelope.body), {
		status: envelope.status,
		headers: new Headers(envelope.headers),
	});
}

function resolveRivetkitPackageRoot(): string {
	const runtimeRequire = createRuntimeRequire();
	const entryPath = runtimeRequire.resolve("rivetkit");
	let current = path.dirname(entryPath);

	while (true) {
		const candidate = path.join(current, "package.json");
		try {
			const packageJsonRaw = requireJsonSync(candidate) as {
				name?: string;
			};
			if (packageJsonRaw?.name === "rivetkit") {
				return current;
			}
		} catch {
			// Continue walking up until package root is found.
		}

		const parent = path.dirname(current);
		if (parent === current) {
			throw new Error("failed to resolve rivetkit package root");
		}
		current = parent;
	}
}

async function resolveDynamicIsolateRuntimeBootstrapEntryPath(): Promise<string> {
	const packageRoot = resolveRivetkitPackageRoot();
	const bootstrapEntryPath = path.join(
		packageRoot,
		"dist",
		"dynamic-isolate-runtime",
		"index.cjs",
	);

	try {
		await stat(bootstrapEntryPath);
	} catch {
		throw new Error(
			"dynamic actor runtime bootstrap is not built. Run `pnpm --filter rivetkit build:dynamic-isolate-runtime` before using dynamicActor.",
		);
	}

	return bootstrapEntryPath;
}

async function resolveDynamicRuntimeModuleAccessCwd(): Promise<string> {
	if (!dynamicRuntimeModuleAccessCwdPromise) {
		dynamicRuntimeModuleAccessCwdPromise = (async () => {
			const packageRoot = resolveRivetkitPackageRoot();
			const sourceDistEntry = path.join(
				packageRoot,
				"dist",
				"tsup",
				"mod.js",
			);
			try {
				await stat(sourceDistEntry);
			} catch {
				throw new Error(
					"dynamic actor runtime requires a built rivetkit package. Run `pnpm --filter rivetkit build` before using dynamicActor.",
				);
			}

			let current = packageRoot;
			let firstNodeModulesCwd: string | undefined;
			while (true) {
				const nodeModulesPath = path.join(current, "node_modules");
				try {
					const nodeModulesStat = await stat(nodeModulesPath);
					if (nodeModulesStat.isDirectory()) {
						if (!firstNodeModulesCwd) {
							firstNodeModulesCwd = current;
						}
						try {
							const pnpmStoreStat = await stat(
								path.join(nodeModulesPath, ".pnpm"),
							);
							if (pnpmStoreStat.isDirectory()) {
								return current;
							}
						} catch {
							// Keep walking up to prefer a node_modules root with
							// a pnpm store directory to avoid symlink escapes.
						}
					}
				} catch {
					// Keep walking up to locate the workspace node_modules.
				}

				const parent = path.dirname(current);
				if (parent === current) {
					if (firstNodeModulesCwd) {
						return firstNodeModulesCwd;
					}
					throw new Error(
						"failed to resolve node_modules root for dynamic actor module access",
					);
				}
				current = parent;
			}
		})();
	}
	return dynamicRuntimeModuleAccessCwdPromise;
}

function createRuntimeRequire(): NodeJS.Require {
	return createRequire(
		path.join(process.cwd(), "__rivetkit_dynamic_require__.cjs"),
	);
}

function requireJsonSync(filePath: string): unknown {
	const runtimeRequire = createRuntimeRequire();
	return runtimeRequire(filePath);
}

async function loadSecureExecModule(): Promise<SecureExecModule> {
	if (!secureExecModulePromise) {
		secureExecModulePromise = (async () => {
			const entryPath = resolveSecureExecEntryPath();
			const entrySpecifier = pathToFileURL(entryPath).href;
			return await nativeDynamicImport<SecureExecModule>(entrySpecifier);
		})();
	}
	return secureExecModulePromise;
}

async function loadIsolatedVmModule(): Promise<IsolatedVmModule> {
	if (!isolatedVmModulePromise) {
		isolatedVmModulePromise = (async () => {
			const entryPath = resolveSecureExecEntryPath();
			const packageDir = resolveSecureExecPackageDir(entryPath);
			const secureExecRequire = createRequire(
				path.join(packageDir, "package.json"),
			);
			// Mirror the sqlite dynamic import pattern by constructing the specifier
			// from parts to avoid static analyzer constant folding.
			const isolatedVmSpecifier = ["isolated", "vm"].join("-");
			return secureExecRequire(isolatedVmSpecifier) as IsolatedVmModule;
		})();
	}
	return isolatedVmModulePromise;
}

/**
 * Resolve an ESM-only package entry by walking up from cwd to find it in
 * node_modules. This handles packages that have "type": "module" and only
 * define "import" in exports (no "require"), which createRequire().resolve()
 * cannot handle.
 */
function resolveEsmPackageEntry(packageName: string): string | undefined {
	let current = process.cwd();
	while (true) {
		const pkgJsonPath = path.join(
			current,
			"node_modules",
			packageName,
			"package.json",
		);
		try {
			const content = readFileSync(pkgJsonPath, "utf-8");
			const pkgJson = JSON.parse(content) as {
				main?: string;
				exports?: Record<string, unknown>;
			};
			const entryRelative =
				(pkgJson.exports?.["."] as { import?: string } | undefined)
					?.import ?? pkgJson.main;
			if (entryRelative) {
				const resolved = path.resolve(
					path.dirname(pkgJsonPath),
					entryRelative,
				);
				// Resolve pnpm symlinks so Node's ESM loader can find the
				// actual file and its co-located dependencies. Use
				// createRequire to dynamically load realpathSync instead of
				// a top-level import, because this module is also loaded
				// inside the sandbox where the fs polyfill lacks it.
				try {
					const runtimeRequire = createRuntimeRequire();
					const nodeFs = runtimeRequire(
						["node", "fs"].join(":"),
					) as {
						realpathSync: (p: string) => string;
					};
					return nodeFs.realpathSync(resolved);
				} catch {
					return resolved;
				}
			}
		} catch {
			// package.json not found at this level, keep walking up
		}
		const parent = path.dirname(current);
		if (parent === current) break;
		current = parent;
	}
	return undefined;
}

function resolveSecureExecEntryPath(): string {
	const explicitSpecifier =
		process.env.RIVETKIT_DYNAMIC_SECURE_EXEC_SPECIFIER;
	const resolver = createRuntimeRequire();
	if (explicitSpecifier) {
		if (explicitSpecifier.startsWith("file://")) {
			return fileURLToPath(explicitSpecifier);
		}
		try {
			return resolver.resolve(explicitSpecifier);
		} catch {
			if (path.isAbsolute(explicitSpecifier)) {
				return explicitSpecifier;
			}
			return path.resolve(explicitSpecifier);
		}
	}

	const packageSpecifiers = [["secure", "exec"].join("-")];
	for (const packageSpecifier of packageSpecifiers) {
		try {
			return resolver.resolve(packageSpecifier);
		} catch {
			// createRequire().resolve() cannot resolve ESM-only packages (packages
			// with "type": "module" and only "import" in exports). Fall back to
			// manually finding the package in node_modules and reading its entry.
			const resolved = resolveEsmPackageEntry(packageSpecifier);
			if (resolved) return resolved;
		}
	}

	const localDistCandidates = [
		path.join(
			process.env.HOME ?? "",
			"secure-exec-rivet/packages/secure-exec/dist/index.js",
		),
	];
	for (const candidatePath of localDistCandidates) {
		try {
			const candidatePackagePath = path.resolve(
				candidatePath,
				"..",
				"..",
				"package.json",
			);
			if (requireJsonSync(candidatePackagePath)) {
				return candidatePath;
			}
		} catch {}
	}

	// Preserve a deterministic fallback for downstream error reporting.
	return localDistCandidates[0];
}

function resolveSecureExecPackageDir(distEntryPath: string): string {
	return path.resolve(distEntryPath, "..", "..");
}

async function nativeDynamicImport<T>(specifier: string): Promise<T> {
	// Try direct dynamic import first because VM-backed test runners may reject
	// import() from Function() with ERR_VM_DYNAMIC_IMPORT_CALLBACK_MISSING.
	try {
		return (await import(specifier)) as T;
	} catch (directError) {
		// Vite SSR can rewrite import() and fail to resolve file:// specifiers
		// outside the project graph. Function() forces the runtime native loader.
		const importer = new Function(
			"moduleSpecifier",
			"return import(moduleSpecifier);",
		) as (moduleSpecifier: string) => Promise<T>;
		try {
			return await importer(specifier);
		} catch {
			throw directError;
		}
	}
}

function buildLockedDownPermissions(): {
	fs: (request: SecureExecFsAccessRequest) => { allow: boolean };
	network: (request: SecureExecNetworkAccessRequest) => { allow: boolean };
	childProcess: () => { allow: boolean };
	env: () => { allow: boolean };
} {
	const sandboxAppRoot = path.resolve(DYNAMIC_SANDBOX_APP_ROOT);
	const sandboxTmpRoot = path.resolve(DYNAMIC_SANDBOX_TMP_ROOT);
	const projectedNodeModules = path.resolve(
		path.posix.join(DYNAMIC_SANDBOX_APP_ROOT, "node_modules"),
	);
	const isPathWithin = (candidate: string, parent: string): boolean => {
		const resolvedCandidate = path.resolve(candidate);
		const resolvedParent = path.resolve(parent);
		return (
			resolvedCandidate === resolvedParent ||
			resolvedCandidate.startsWith(`${resolvedParent}${path.sep}`)
		);
	};
	const isReadOnlyFsOp = (
		operation: SecureExecFsAccessRequest["op"],
	): boolean => {
		return (
			operation === "read" ||
			operation === "readdir" ||
			operation === "stat" ||
			operation === "exists"
		);
	};

	return {
		fs: (request: SecureExecFsAccessRequest) => {
			if (isPathWithin(request.path, projectedNodeModules)) {
				return {
					allow: isReadOnlyFsOp(request.op),
				};
			}

			return {
				allow:
					isPathWithin(request.path, sandboxAppRoot) ||
					isPathWithin(request.path, sandboxTmpRoot),
			};
		},
		network: (_request: SecureExecNetworkAccessRequest) => ({
			allow: false,
		}),
		childProcess: () => ({ allow: false }),
		// Dynamic actors only receive explicitly injected env vars from
		// processConfig.env, so this does not expose host environment values.
		env: () => ({ allow: true }),
	};
}

async function normalizeBinaryPayload(
	data: ArrayBufferLike | Blob | ArrayBufferView,
): Promise<Uint8Array> {
	if (data instanceof Blob) {
		return new Uint8Array(await data.arrayBuffer());
	}
	if (ArrayBuffer.isView(data)) {
		return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
	}
	return new Uint8Array(data);
}

function copyUint8ArrayToArrayBuffer(value: Uint8Array): ArrayBuffer {
	return value.buffer.slice(
		value.byteOffset,
		value.byteOffset + value.byteLength,
	) as ArrayBuffer;
}

async function materializeDynamicSource(
	loadResult: NormalizedDynamicActorLoadResult,
): Promise<MaterializedDynamicSource> {
	switch (loadResult.sourceFormat) {
		case "esm-js": {
			const sourceEntry = "dynamic-source.mjs";
			const sourcePath = path.posix.join(
				DYNAMIC_SANDBOX_APP_ROOT,
				sourceEntry,
			);
			return {
				sourcePath,
				sourceCode: loadResult.source,
				sourceEntry,
				sourceFormat: loadResult.sourceFormat,
			};
		}
		case "commonjs-js": {
			const sourceEntry = "dynamic-source.cjs";
			const sourcePath = path.posix.join(
				DYNAMIC_SANDBOX_APP_ROOT,
				sourceEntry,
			);
			return {
				sourcePath,
				sourceCode: loadResult.source,
				sourceEntry,
				sourceFormat: loadResult.sourceFormat,
			};
		}
		case "esm-ts":
		case "commonjs-ts": {
			const isEsm = loadResult.sourceFormat === "esm-ts";
			const compileResult = await compileActorSource({
				source: loadResult.source,
				format: isEsm ? "esm" : "commonjs",
				typecheck: false,
			});
			if (!compileResult.success || !compileResult.js) {
				const messages = compileResult.diagnostics
					.map((d) => d.message)
					.join("\n");
				throw new Error(
					`TypeScript compilation failed:\n${messages}`,
				);
			}
			const jsFormat: DynamicSourceFormat = isEsm
				? "esm-js"
				: "commonjs-js";
			const sourceEntry = isEsm
				? "dynamic-source.mjs"
				: "dynamic-source.cjs";
			const sourcePath = path.posix.join(
				DYNAMIC_SANDBOX_APP_ROOT,
				sourceEntry,
			);
			return {
				sourcePath,
				sourceCode: compileResult.js,
				sourceEntry,
				sourceFormat: jsFormat,
			};
		}
		default: {
			throw new Error(
				`unsupported dynamic source format: ${String(loadResult.sourceFormat)}`,
			);
		}
	}
}
