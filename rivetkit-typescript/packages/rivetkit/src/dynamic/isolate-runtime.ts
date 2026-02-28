import { createRequire } from "node:module";
import {
	cp,
	mkdir,
	rename,
	readdir,
	readFile,
	rm,
	stat,
	writeFile,
} from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { VirtualWebSocket } from "@rivetkit/virtual-websocket";
import type { ActorDriver } from "@/actor/driver";
import type { ActorKey } from "@/actor/mod";
import type { Encoding } from "@/actor/protocol/serde";
import { getLogger } from "@/common/log";
import { stringifyError } from "@/common/utils";
import type { UniversalWebSocket } from "@/common/websocket-interface";
import type { AnyClient } from "@/client/client";
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
	createDynamicActorLoaderContext,
	type DynamicActorLoader,
	type DynamicActorLoadResult,
} from "./internal";

export type { DynamicHibernatingWebSocketMetadata } from "./runtime-bridge";

const DYNAMIC_RUNTIME_ROOT = path.join(
	process.env.TMPDIR ?? "/tmp",
	`rivetkit-dynamic-actors-${process.pid}`,
);
const DYNAMIC_RUNTIME_ACTORS_ROOT = path.join(DYNAMIC_RUNTIME_ROOT, "actors");
const DYNAMIC_RUNTIME_NODE_MODULES = path.join(
	DYNAMIC_RUNTIME_ROOT,
	"node_modules",
);

let dynamicRuntimeSetupPromise: Promise<void> | undefined;
let secureExecModulePromise: Promise<SecureExecModule> | undefined;
let isolatedVmModulePromise: Promise<IsolatedVmModule> | undefined;

function logger() {
	return getLogger("dynamic-actor");
}

interface SecureExecModule {
	NodeProcess: new (options: Record<string, unknown>) => NodeProcessLike;
	NodeFileSystem: new () => unknown;
	createNodeDriver?: (options: Record<string, unknown>) => unknown;
}

interface IsolatedVmModule {
	Reference: new <T>(value: T) => ReferenceLike<T>;
	ExternalCopy: new <T>(value: T) => {
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
	__unsafeCreateContext(options?: Record<string, unknown>): Promise<ContextLike>;
	dispose(): void;
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
	dispose: ReferenceLike<() => Promise<boolean>>;
}

interface NormalizedDynamicActorLoadResult extends DynamicActorLoadResult {
	sourceFormat: DynamicSourceFormat;
}

interface MaterializedDynamicSource {
	sourcePath: string;
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
	#runtimeDir: string;

	#nodeProcess: NodeProcessLike | undefined;
	#context: ContextLike | undefined;
	#refs: DynamicRuntimeRefs | undefined;

	#referenceHandles: Array<{ release?: () => void }> = [];
	#webSocketSessions = new Map<number, HostWebSocketSession>();
	#sessionIdsByWebSocket = new WeakMap<UniversalWebSocket, number>();
	#nextWebSocketSessionId = 1;
	#started = false;
	#disposed = false;

	constructor(config: DynamicActorIsolateRuntimeConfig) {
		this.#config = config;
		this.#runtimeDir = path.join(DYNAMIC_RUNTIME_ACTORS_ROOT, config.actorId);
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
		await ensureDynamicRuntimeNodeModules();
		logger().debug({
			msg: "dynamic runtime node_modules ready",
			actorId: this.#config.actorId,
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

		await rm(this.#runtimeDir, { recursive: true, force: true });
		await mkdir(this.#runtimeDir, { recursive: true });

		const materializedSource = await materializeDynamicSource(
			this.#runtimeDir,
			normalizedLoadResult,
		);
		logger().debug({
			msg: "dynamic runtime source written",
			actorId: this.#config.actorId,
			sourcePath: materializedSource.sourcePath,
			sourceEntry: materializedSource.sourceEntry,
			sourceFormat: materializedSource.sourceFormat,
		});

		const bootstrapSourcePath = await resolveDynamicIsolateRuntimeBootstrapEntryPath();
		const bootstrapPath = path.join(this.#runtimeDir, "dynamic-bootstrap.cjs");
		await cp(bootstrapSourcePath, bootstrapPath);
		logger().debug({
			msg: "dynamic runtime bootstrap written",
			actorId: this.#config.actorId,
			bootstrapSourcePath,
			bootstrapPath,
		});

		const secureExec = await loadSecureExecModule();
		const ivm = await loadIsolatedVmModule();

		const permissions = buildLockedDownPermissions(DYNAMIC_RUNTIME_ROOT);
		const driver = buildLockedDownSandboxDriver(secureExec, permissions);
		const sandboxHomeDir = path.join(this.#runtimeDir, ".sandbox-home");
		const sandboxDataDir = path.join(sandboxHomeDir, ".local", "share");
		const sandboxCacheDir = path.join(sandboxHomeDir, ".cache");
		const sandboxTmpDir = path.join(sandboxHomeDir, "tmp");
		await mkdir(sandboxDataDir, { recursive: true });
		await mkdir(sandboxCacheDir, { recursive: true });
		await mkdir(sandboxTmpDir, { recursive: true });

		this.#nodeProcess = new secureExec.NodeProcess({
			driver,
			processConfig: {
				cwd: this.#runtimeDir,
				env: {
					HOME: sandboxHomeDir,
					XDG_DATA_HOME: sandboxDataDir,
					XDG_CACHE_HOME: sandboxCacheDir,
					TMPDIR: sandboxTmpDir,
					RIVET_EXPOSE_ERRORS: "1",
				},
			},
			osConfig: {
				homedir: sandboxHomeDir,
				tmpdir: sandboxTmpDir,
			},
			memoryLimit: normalizedLoadResult.nodeProcess?.memoryLimit,
			cpuTimeLimitMs: normalizedLoadResult.nodeProcess?.cpuTimeLimitMs,
		});

		this.#context = await this.#nodeProcess.__unsafeCreateContext({
			cwd: this.#runtimeDir,
			filePath: path.join(this.#runtimeDir, "dynamic-host-init.cjs"),
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
		await this.#loadBootstrap(bootstrapPath);
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
					void this.#closeWebSocketMessage(session.id, code, reason);
				},
			}),
			dispatchReady: false,
			pendingDispatches: [],
			pendingMessages: [],
		};
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
						isRestoringHibernatable: options.isRestoringHibernatable,
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
			session.websocket.triggerClose(1011, "dynamic.websocket.open_failed", false);
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
			session.readyState = 3;
			session.websocket.triggerClose(1001, "dynamic.runtime.disposed", false);
		}
		this.#webSocketSessions.clear();

		if (this.#refs) {
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
						[
							new Uint8Array(key),
							new Uint8Array(value),
						] as [Uint8Array, Uint8Array],
				);
				await this.#config.actorDriver.kvBatchPut(actorId, decodedEntries);
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
					value ? copyUint8ArrayToArrayBuffer(value) : null
					),
				);
			},
		);
		const kvBatchDeleteRef = makeRef(
			async (actorId: string, keys: ArrayBuffer[]): Promise<void> => {
				const decodedKeys = keys.map((key) => new Uint8Array(key));
				await this.#config.actorDriver.kvBatchDelete(actorId, decodedKeys);
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
				const accessor = (this.#config.inlineClient as Record<string, any>)[
					input.actorName
				];
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
					typeof this.#config.actorDriver.ackHibernatableWebSocketMessage !==
					"function"
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
		const logRef = makeRef((level: "debug" | "warn", message: string): void => {
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
		});

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
		const bootstrapScript = await this.#nodeProcess.__unsafeIsoalte.compileScript(
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
				filename: path.join(this.#runtimeDir, "dynamic-bootstrap-entry.cjs"),
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
			return await getRef<T>(DYNAMIC_ISOLATE_EXPORT_GLOBAL_KEYS[exportName]);
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
			dispose: await getExportRef("dynamicDisposeEnvelope"),
		};
	}

	#handleIsolateDispatch(payload: IsolateDispatchPayload): void {
		logger().debug({
			msg: "dynamic websocket dispatch",
			actorId: this.#config.actorId,
			payloadType: payload.type,
			sessionId: payload.sessionId,
		});
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

	const sourceFormat = loadResult.sourceFormat ?? "typescript";
	if (
		sourceFormat !== "typescript" &&
		sourceFormat !== "commonjs-js" &&
		sourceFormat !== "esm-js"
	) {
		throw new Error(
			"dynamic actor loader returned unsupported `sourceFormat`. Expected `typescript`, `commonjs-js`, or `esm-js`.",
		);
	}

	return {
		...loadResult,
		sourceFormat,
	};
}

async function requestToEnvelope(request: Request): Promise<FetchEnvelopeInput> {
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

/**
 * Prepares a dedicated runtime node_modules tree for dynamic actor isolates.
 *
 * This is currently a temporary compatibility layer while secure-exec still
 * has CJS and ESM interop gaps for some RivetKit transitive dependencies. We
 * copy RivetKit and its dependency closure into a runtime directory, then patch
 * a subset of packages to CJS safe artifacts.
 *
 * TODO: Remove this materialization and patching path once secure-exec can
 * load the required packages directly without source rewriting.
 */
async function ensureDynamicRuntimeNodeModules(): Promise<void> {
	if (!dynamicRuntimeSetupPromise) {
		dynamicRuntimeSetupPromise = (async () => {
			await mkdir(DYNAMIC_RUNTIME_ACTORS_ROOT, { recursive: true });
			const packageRoot = resolveRivetkitPackageRoot();
			const sourceDistEntry = path.join(packageRoot, "dist", "tsup", "mod.js");

			try {
				await stat(sourceDistEntry);
			} catch {
				throw new Error(
					"dynamic actor runtime requires a built rivetkit package. Run `pnpm --filter rivetkit build` before using dynamicActor.",
				);
			}
			const sourceDistStat = await stat(sourceDistEntry);
			void sourceDistStat;

			const stagingNodeModulesPath = `${DYNAMIC_RUNTIME_NODE_MODULES}.tmp.${process.pid}.${Date.now()}`;
			await rm(stagingNodeModulesPath, {
				recursive: true,
				force: true,
			});
			await mkdir(stagingNodeModulesPath, { recursive: true });
			try {
				// Materialize only rivetkit and its transitive dependency closure into
				// an isolated runtime node_modules tree.
				await materializeRuntimeRivetkitPackage(
					packageRoot,
					stagingNodeModulesPath,
				);
				await materializeRuntimeDependencyClosure(
					packageRoot,
					stagingNodeModulesPath,
				);
				await patchRuntimeOnChangePackageToCommonJs(
					stagingNodeModulesPath,
				);
				await patchRuntimeNanoeventsPackageToCommonJs(
					stagingNodeModulesPath,
				);
				await patchRuntimePRetryPackageToCommonJs(
					stagingNodeModulesPath,
				);
				await patchRuntimeIsNetworkErrorPackageToCommonJs(
					stagingNodeModulesPath,
				);
				await patchRuntimeGetPortPackageToCommonJs(
					stagingNodeModulesPath,
				);
				await patchRuntimeTracesPackageToNoop(stagingNodeModulesPath);

				await rm(DYNAMIC_RUNTIME_NODE_MODULES, {
					recursive: true,
					force: true,
				});
				await rename(stagingNodeModulesPath, DYNAMIC_RUNTIME_NODE_MODULES);
			} catch (error) {
				await rm(stagingNodeModulesPath, {
					recursive: true,
					force: true,
				});
				throw error;
			}
		})();
	}
	return dynamicRuntimeSetupPromise;
}

async function materializeRuntimeRivetkitPackage(
	packageRoot: string,
	runtimeNodeModulesRoot: string,
): Promise<void> {
	const runtimeRivetkitRoot = path.join(runtimeNodeModulesRoot, "rivetkit");
	await rm(runtimeRivetkitRoot, { recursive: true, force: true });
	await mkdir(runtimeRivetkitRoot, { recursive: true });

	await cp(
		path.join(packageRoot, "package.json"),
		path.join(runtimeRivetkitRoot, "package.json"),
	);
	await cp(
		path.join(packageRoot, "dist"),
		path.join(runtimeRivetkitRoot, "dist"),
		{ recursive: true },
	);
	await patchRivetkitCjsArtifacts(runtimeRivetkitRoot);
}

async function patchRivetkitCjsArtifacts(runtimeRivetkitRoot: string): Promise<void> {
	const tsupDistDir = path.join(runtimeRivetkitRoot, "dist", "tsup");

	const cjsFiles: string[] = [];
	const walk = async (currentDir: string): Promise<void> => {
		let entries: string[];
		try {
			entries = await readdir(currentDir);
		} catch {
			return;
		}

		for (const entry of entries) {
			const entryPath = path.join(currentDir, entry);
			let entryStat;
			try {
				entryStat = await stat(entryPath);
			} catch {
				continue;
			}

			if (entryStat.isDirectory()) {
				await walk(entryPath);
				continue;
			}

			if (entryStat.isFile() && entryPath.endsWith(".cjs")) {
				cjsFiles.push(entryPath);
			}
		}
	};

	await walk(tsupDistDir);

	for (const filePath of cjsFiles) {
		let sourceText: string;
		try {
			sourceText = await readFile(filePath, "utf8");
		} catch {
			continue;
		}

		const patchedText = sourceText
			.replaceAll("import.meta.url", "__filename")
			.replaceAll("url.fileURLToPath(__filename)", "__filename")
			.replaceAll("fileURLToPath(__filename)", "__filename")
			.replaceAll("fileURLToPath.call(void 0, __filename)", "__filename");

		if (patchedText !== sourceText) {
			await writeFile(filePath, patchedText, "utf8");
		}
	}
}

async function materializeRuntimeDependencyClosure(
	packageRoot: string,
	runtimeNodeModulesRoot: string,
): Promise<void> {
	const queue: Array<{ name: string; issuerPackageRoot: string }> = [];
	const visited = new Set<string>();

	for (const dependencyName of getRivetkitDependencyNames(packageRoot)) {
		queue.push({
			name: dependencyName,
			issuerPackageRoot: packageRoot,
		});
	}
	queue.push({
		name: "rivetkit",
		issuerPackageRoot: packageRoot,
	});

	while (queue.length > 0) {
		const current = queue.shift()!;
		const sourcePackageRoot = resolveDependencyPackageRoot(
			current.issuerPackageRoot,
			current.name,
		);
		if (!sourcePackageRoot) {
			continue;
		}

		const visitKey = `${current.name}:${sourcePackageRoot}`;
		if (visited.has(visitKey)) {
			continue;
		}
		visited.add(visitKey);

		if (current.name !== "rivetkit") {
			await materializeRuntimePackageIfMissing(
				current.name,
				sourcePackageRoot,
				runtimeNodeModulesRoot,
			);
		}

		for (const nestedDependencyName of getPackageDependencyNames(
			sourcePackageRoot,
		)) {
			queue.push({
				name: nestedDependencyName,
				issuerPackageRoot: sourcePackageRoot,
			});
		}
	}
}

async function patchRuntimeOnChangePackageToCommonJs(
	runtimeNodeModulesRoot: string,
): Promise<void> {
	const packageRoot = path.join(
		runtimeNodeModulesRoot,
		"@rivetkit",
		"on-change",
	);
	const packageSourceRoot = path.join(packageRoot, "source");
	try {
		await stat(packageSourceRoot);
	} catch {
		return;
	}

	// TODO: Temporary compatibility patch until secure-exec can safely load
	// ESM modules from CJS require calls without deadlocking.
	const sourceFiles = await collectJavaScriptFiles(packageSourceRoot);
	for (const sourceFilePath of sourceFiles) {
		const sourceText = await readFile(sourceFilePath, "utf8");
		const transpiledText = await transpileToCommonJs(
			sourceText,
			sourceFilePath,
		);
		await writeFile(sourceFilePath, transpiledText, "utf8");
	}

	const packageIndexPath = path.join(packageSourceRoot, "index.js");
	try {
		const indexSourceText = await readFile(packageIndexPath, "utf8");
		if (indexSourceText.includes("exports.default = onChange;")) {
			const interopShim = `
module.exports = onChange;
module.exports.default = onChange;
module.exports.target = onChange.target;
module.exports.unsubscribe = onChange.unsubscribe;
`;
			await writeFile(
				packageIndexPath,
				`${indexSourceText.trimEnd()}\n${interopShim}`,
				"utf8",
			);
		}
	} catch {
		// Ignore missing source index in staged dependencies.
	}

	await patchPackageTypeToCommonJs(packageRoot);
}

async function patchRuntimeNanoeventsPackageToCommonJs(
	runtimeNodeModulesRoot: string,
): Promise<void> {
	const packageSourceRoot = path.join(
		runtimeNodeModulesRoot,
		"nanoevents",
	);
	try {
		await stat(packageSourceRoot);
	} catch {
		return;
	}

	// TODO: Temporary compatibility patch until secure-exec can safely load
	// ESM package entrypoints from CJS require calls.
	const sourceFiles = await collectJavaScriptFiles(packageSourceRoot);
	for (const sourceFilePath of sourceFiles) {
		const sourceText = await readFile(sourceFilePath, "utf8");
		const transpiledText = await transpileToCommonJs(
			sourceText,
			sourceFilePath,
		);
		await writeFile(sourceFilePath, transpiledText, "utf8");
	}
	await patchPackageTypeToCommonJs(packageSourceRoot);
}

async function patchRuntimePRetryPackageToCommonJs(
	runtimeNodeModulesRoot: string,
): Promise<void> {
	const packageSourceRoot = path.join(
		runtimeNodeModulesRoot,
		"p-retry",
	);
	try {
		await stat(packageSourceRoot);
	} catch {
		return;
	}

	// TODO: Temporary compatibility patch until secure-exec can safely load
	// ESM package entrypoints from CJS require calls.
	const sourceFiles = await collectJavaScriptFiles(packageSourceRoot);
	for (const sourceFilePath of sourceFiles) {
		const sourceText = await readFile(sourceFilePath, "utf8");
		const transpiledText = await transpileToCommonJs(
			sourceText,
			sourceFilePath,
		);
		await writeFile(sourceFilePath, transpiledText, "utf8");
	}
	await patchPackageTypeToCommonJs(packageSourceRoot);
}

async function patchRuntimeIsNetworkErrorPackageToCommonJs(
	runtimeNodeModulesRoot: string,
): Promise<void> {
	const packageSourceRoot = path.join(
		runtimeNodeModulesRoot,
		"is-network-error",
	);
	try {
		await stat(packageSourceRoot);
	} catch {
		return;
	}

	// TODO: Temporary compatibility patch until secure-exec can safely load
	// ESM package entrypoints from CJS require calls.
	const sourceFiles = await collectJavaScriptFiles(packageSourceRoot);
	for (const sourceFilePath of sourceFiles) {
		const sourceText = await readFile(sourceFilePath, "utf8");
		const transpiledText = await transpileToCommonJs(
			sourceText,
			sourceFilePath,
		);
		await writeFile(sourceFilePath, transpiledText, "utf8");
	}
	await patchPackageTypeToCommonJs(packageSourceRoot);
}

async function patchRuntimeGetPortPackageToCommonJs(
	runtimeNodeModulesRoot: string,
): Promise<void> {
	const packageSourceRoot = path.join(
		runtimeNodeModulesRoot,
		"get-port",
	);
	try {
		await stat(packageSourceRoot);
	} catch {
		return;
	}

	// TODO: Temporary compatibility patch until secure-exec can safely load
	// ESM package entrypoints from CJS require calls.
	const sourceFiles = await collectJavaScriptFiles(packageSourceRoot);
	for (const sourceFilePath of sourceFiles) {
		const sourceText = await readFile(sourceFilePath, "utf8");
		const transpiledText = await transpileToCommonJs(
			sourceText,
			sourceFilePath,
		);
		await writeFile(sourceFilePath, transpiledText, "utf8");
	}
	await patchPackageTypeToCommonJs(packageSourceRoot);
}

async function patchPackageTypeToCommonJs(
	packageRoot: string,
): Promise<void> {
	const packageJsonPath = path.join(packageRoot, "package.json");
	try {
		const packageJsonText = await readFile(packageJsonPath, "utf8");
		const packageJson = JSON.parse(packageJsonText) as Record<string, unknown>;
		packageJson.type = "commonjs";
		await writeFile(
			packageJsonPath,
			`${JSON.stringify(packageJson, null, 2)}\n`,
			"utf8",
		);
	} catch {
		// Ignore missing or invalid package metadata in staged dependencies.
	}
}

async function patchRuntimeTracesPackageToNoop(
	runtimeNodeModulesRoot: string,
): Promise<void> {
	const tracesIndexPath = path.join(
		runtimeNodeModulesRoot,
		"@rivetkit",
		"traces",
		"dist",
		"tsup",
		"index.cjs",
	);
	try {
		await stat(tracesIndexPath);
	} catch {
		return;
	}

	// TODO: Temporary compatibility patch until secure-exec can load the full
	// traces package (including async_hooks and ESM transitive dependencies)
	// reliably from CJS entrypoints in dynamic actor isolates.
	await writeFile(
		tracesIndexPath,
		`"use strict";

const NOOP_SPAN = {
	spanId: new Uint8Array(8),
	traceId: new Uint8Array(16),
	isActive: () => false,
};

function createEmptyOtlpExport() {
	return {
		resourceSpans: [
			{
				scopeSpans: [{ spans: [] }],
			},
		],
	};
}

function createNoopTraces() {
	return {
		startSpan: () => NOOP_SPAN,
		updateSpan: () => {},
		setAttributes: () => {},
		setStatus: () => {},
		endSpan: () => {},
		emitEvent: () => {},
		withSpan: (_handle, fn) => fn(),
		getCurrentSpan: () => null,
		flush: async () => false,
		readRange: async () => ({
			otlp: createEmptyOtlpExport(),
			clamped: false,
		}),
		readRangeWire: async (options) => ({
			startTimeMs: BigInt(options.startMs),
			endTimeMs: BigInt(options.endMs),
			limit: Math.max(0, Math.min(0xffff_ffff, Math.floor(options.limit))),
			clamped: false,
			baseChunks: [],
			chunks: [],
		}),
	};
}

function createTraces() {
	return createNoopTraces();
}

module.exports = {
	createTraces,
	createNoopTraces,
};
`,
		"utf8",
	);
}

async function collectJavaScriptFiles(rootDir: string): Promise<string[]> {
	const output: string[] = [];
	const entries = await readdir(rootDir, { withFileTypes: true });
	for (const entry of entries) {
		const entryPath = path.join(rootDir, entry.name);
		if (entry.isDirectory()) {
			const nested = await collectJavaScriptFiles(entryPath);
			output.push(...nested);
			continue;
		}
		if (entry.isFile() && entry.name.endsWith(".js")) {
			output.push(entryPath);
		}
	}
	return output;
}

async function materializeRuntimePackageIfMissing(
	dependencyName: string,
	sourcePackageRoot: string,
	runtimeNodeModulesRoot: string,
): Promise<void> {
	const runtimeDependencyRoot = path.join(
		runtimeNodeModulesRoot,
		dependencyName,
	);
	try {
		await stat(runtimeDependencyRoot);
		return;
	} catch {
		// Dependency missing from runtime node_modules; materialize it now.
	}

	await mkdir(path.dirname(runtimeDependencyRoot), { recursive: true });
	await cp(sourcePackageRoot, runtimeDependencyRoot, {
		recursive: true,
		dereference: true,
		filter: (entryPath) => path.basename(entryPath) !== "node_modules",
	});
}

function resolveDependencyPackageRoot(
	issuerPackageRoot: string,
	dependencyName: string,
): string | undefined {
	let resolvedPath: string | undefined;
	const issuerRequire = createRequire(path.join(issuerPackageRoot, "package.json"));
	try {
		resolvedPath = issuerRequire.resolve(
			`${dependencyName}/package.json`,
		);
	} catch {
		try {
			resolvedPath = issuerRequire.resolve(dependencyName);
		} catch {
			return undefined;
		}
	}

	return (
		findPackageRootForDependency(resolvedPath, dependencyName) ??
		findNearestPackageRoot(resolvedPath)
	);
}

function findNearestPackageRoot(entryPath: string): string | undefined {
	let current = path.dirname(entryPath);
	while (true) {
		const candidatePackageJson = path.join(current, "package.json");
		try {
			requireJsonSync(candidatePackageJson);
			return current;
		} catch {
			// Continue walking upward.
		}

		const parent = path.dirname(current);
		if (parent === current) {
			return undefined;
		}
		current = parent;
	}
}

function findPackageRootForDependency(
	entryPath: string,
	dependencyName: string,
): string | undefined {
	let current = path.dirname(entryPath);
	let fallback: string | undefined;
	while (true) {
		const candidatePackageJson = path.join(current, "package.json");
		try {
			const packageJson = requireJsonSync(candidatePackageJson) as {
				name?: string;
			};
			if (!fallback) {
				fallback = current;
			}
			if (packageJson?.name === dependencyName) {
				return current;
			}
		} catch {
			// Continue walking upward.
		}

		const parent = path.dirname(current);
		if (parent === current) {
			return fallback;
		}
		current = parent;
	}
}

function getPackageDependencyNames(packageRoot: string): string[] {
	try {
		const packageJson = requireJsonSync(
			path.join(packageRoot, "package.json"),
		) as {
			dependencies?: Record<string, string>;
			optionalDependencies?: Record<string, string>;
			peerDependencies?: Record<string, string>;
		};
		return [
			...Object.keys(packageJson.dependencies ?? {}),
			...Object.keys(packageJson.optionalDependencies ?? {}),
			...Object.keys(packageJson.peerDependencies ?? {}),
		];
	} catch {
		return [];
	}
}

function getRivetkitDependencyNames(packageRoot: string): string[] {
	try {
		const packageJson = requireJsonSync(
			path.join(packageRoot, "package.json"),
		) as {
			dependencies?: Record<string, string>;
		};
		return Object.keys(packageJson.dependencies ?? {});
	} catch {
		return [];
	}
}

function createRuntimeRequire(): NodeJS.Require {
	return createRequire(path.join(process.cwd(), "__rivetkit_dynamic_require__.cjs"));
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
			const secureExecRequire = createRequire(path.join(packageDir, "package.json"));
			// Mirror the sqlite dynamic import pattern by constructing the specifier
			// from parts to avoid static analyzer constant folding.
			const isolatedVmSpecifier = ["isolated", "vm"].join("-");
			return secureExecRequire(isolatedVmSpecifier) as IsolatedVmModule;
		})();
	}
	return isolatedVmModulePromise;
}

function resolveSecureExecEntryPath(): string {
	const explicitSpecifier = process.env.RIVETKIT_DYNAMIC_SECURE_EXEC_SPECIFIER;
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

	const packageSpecifiers = [
		["secure", "exec"].join("-"),
		["sandboxed", "node"].join("-"),
	];
	for (const packageSpecifier of packageSpecifiers) {
		try {
			return resolver.resolve(packageSpecifier);
		} catch {}
	}

	const localDistCandidates = [
		path.join(
			process.env.HOME ?? "",
			"secure-exec-rivet/packages/secure-exec/dist/index.js",
		),
		path.join(
			process.env.HOME ?? "",
			"secure-exec-rivet/packages/sandboxed-node/dist/index.js",
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

function buildLockedDownPermissions(rootPath: string): {
	fs: (request: SecureExecFsAccessRequest) => { allow: boolean };
	network: (request: SecureExecNetworkAccessRequest) => { allow: boolean };
	childProcess: () => { allow: boolean };
	env: () => { allow: boolean };
} {
	const resolvedRoot = path.resolve(rootPath);
	const fallbackNodeModules = path.join(path.dirname(resolvedRoot), "node_modules");
	const rootNodeModules = path.join(path.sep, "node_modules");
	const isPathWithin = (candidate: string, parent: string): boolean => {
		const resolvedCandidate = path.resolve(candidate);
		const resolvedParent = path.resolve(parent);
		return (
			resolvedCandidate === resolvedParent ||
			resolvedCandidate.startsWith(`${resolvedParent}${path.sep}`)
		);
	};
	const isReadOnlyFsOp = (operation: SecureExecFsAccessRequest["op"]): boolean => {
		return (
			operation === "read" ||
			operation === "readdir" ||
			operation === "stat" ||
			operation === "exists"
		);
	};

	return {
		fs: (request: SecureExecFsAccessRequest) => ({
			allow:
				isPathWithin(request.path, resolvedRoot) ||
				(isReadOnlyFsOp(request.op) &&
					(isPathWithin(request.path, fallbackNodeModules) ||
						isPathWithin(request.path, rootNodeModules))),
		}),
		network: (_request: SecureExecNetworkAccessRequest) => ({ allow: false }),
		childProcess: () => ({ allow: false }),
		// Dynamic actors only receive explicitly injected env vars from
		// processConfig.env, so this does not expose host environment values.
		env: () => ({ allow: true }),
	};
}

function buildLockedDownSandboxDriver(
	secureExec: SecureExecModule,
	permissions: ReturnType<typeof buildLockedDownPermissions>,
): Record<string, unknown> {
	return {
		filesystem: new secureExec.NodeFileSystem(),
		permissions,
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
	runtimeDir: string,
	loadResult: NormalizedDynamicActorLoadResult,
): Promise<MaterializedDynamicSource> {
	switch (loadResult.sourceFormat) {
		case "esm-js": {
			const sourceEntry = "dynamic-source.mjs";
			const sourcePath = path.join(runtimeDir, sourceEntry);
			await writeFile(sourcePath, loadResult.source, "utf8");
			return {
				sourcePath,
				sourceEntry,
				sourceFormat: loadResult.sourceFormat,
			};
		}
		case "commonjs-js": {
			const sourceEntry = "dynamic-source.cjs";
			const sourcePath = path.join(runtimeDir, sourceEntry);
			await writeFile(sourcePath, loadResult.source, "utf8");
			return {
				sourcePath,
				sourceEntry,
				sourceFormat: loadResult.sourceFormat,
			};
		}
		default: {
			const sourceEntry = "dynamic-source.cjs";
			const sourcePath = path.join(runtimeDir, sourceEntry);
			const sourceCode = await transpileToCommonJs(
				loadResult.source,
				"dynamic-source.ts",
			);
			await writeFile(sourcePath, sourceCode, "utf8");
			return {
				sourcePath,
				sourceEntry,
				sourceFormat: loadResult.sourceFormat,
			};
		}
	}
}

async function transpileToCommonJs(
	source: string,
	fileName: string,
): Promise<string> {
	const ts = (await import("typescript")) as typeof import("typescript");
	const result = ts.transpileModule(source, {
		fileName,
		compilerOptions: {
			target: ts.ScriptTarget.ES2022,
			module: ts.ModuleKind.CommonJS,
			esModuleInterop: true,
			allowSyntheticDefaultImports: true,
		},
	});
	return result.outputText;
}
