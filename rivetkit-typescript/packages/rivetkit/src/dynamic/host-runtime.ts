import { createRequire } from "node:module";
import {
	cp,
	mkdir,
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
	createDynamicActorLoaderContext,
	type DynamicActorLoader,
	type DynamicActorLoadResult,
} from "./internal";

const DYNAMIC_RUNTIME_ROOT = path.join(
	process.env.TMPDIR ?? "/tmp",
	"rivetkit-dynamic-actors",
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

interface FetchEnvelopeInput {
	url: string;
	method: string;
	headers: Record<string, string>;
	bodyBase64?: string;
}

interface FetchEnvelopeOutput {
	status: number;
	headers: Array<[string, string]>;
	bodyBase64: string;
}

interface WebSocketOpenEnvelopeInput {
	sessionId: number;
	path: string;
	encoding: Encoding;
	params: unknown;
	headers?: Record<string, string>;
	gatewayIdBase64?: string;
	requestIdBase64?: string;
	isHibernatable?: boolean;
	isRestoringHibernatable?: boolean;
}

interface WebSocketSendEnvelopeInput {
	sessionId: number;
	kind: "text" | "binary";
	text?: string;
	dataBase64?: string;
	rivetMessageIndex?: number;
}

interface WebSocketCloseEnvelopeInput {
	sessionId: number;
	code?: number;
	reason?: string;
}

type IsolateDispatchPayload =
	| {
			type: "open";
			sessionId: number;
	  }
	| {
			type: "message";
			sessionId: number;
			kind: "text" | "binary";
			text?: string;
			dataBase64?: string;
			rivetMessageIndex?: number;
	  }
	| {
			type: "close";
			sessionId: number;
			code?: number;
			reason?: string;
			wasClean?: boolean;
	  }
	| {
			type: "error";
			sessionId: number;
			message?: string;
	  };

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

interface DynamicActorHostRuntimeConfig {
	actorId: string;
	actorName: string;
	actorKey: ActorKey;
	input: unknown;
	region: string;
	loader: DynamicActorLoader;
	actorDriver: ActorDriver;
	inlineClient: AnyClient;
}

export interface DynamicWebSocketOpenOptions {
	headers?: Record<string, string>;
	gatewayId?: ArrayBuffer;
	requestId?: ArrayBuffer;
	isHibernatable?: boolean;
	isRestoringHibernatable?: boolean;
}

export interface DynamicHibernatingWebSocketMetadata {
	gatewayId: ArrayBuffer;
	requestId: ArrayBuffer;
	serverMessageIndex: number;
	clientMessageIndex: number;
	path: string;
	headers: Record<string, string>;
}

export class DynamicActorHostRuntime {
	#config: DynamicActorHostRuntimeConfig;
	#runtimeDir: string;

	#nodeProcess: NodeProcessLike | undefined;
	#context: ContextLike | undefined;

	#fetchRef: ReferenceLike<(input: FetchEnvelopeInput) => Promise<FetchEnvelopeOutput>> | undefined;
	#dispatchAlarmRef: ReferenceLike<() => Promise<boolean>> | undefined;
	#stopRef: ReferenceLike<(mode: "sleep" | "destroy") => Promise<boolean>> | undefined;
	#openWebSocketRef:
		| ReferenceLike<(input: WebSocketOpenEnvelopeInput) => Promise<boolean>>
		| undefined;
	#sendWebSocketRef:
		| ReferenceLike<(input: WebSocketSendEnvelopeInput) => Promise<boolean>>
		| undefined;
	#closeWebSocketRef:
		| ReferenceLike<(input: WebSocketCloseEnvelopeInput) => Promise<boolean>>
		| undefined;
	#getHibernatingWebSocketsRef:
		| ReferenceLike<
				() => Promise<Array<DynamicHibernatingWebSocketMetadata>>
		  >
		| undefined;
	#disposeRef: ReferenceLike<() => Promise<boolean>> | undefined;
	#startSleepRef: ReferenceLike<(actorId: string) => void> | undefined;
	#startDestroyRef: ReferenceLike<(actorId: string) => void> | undefined;

	#referenceHandles: Array<{ release?: () => void }> = [];
	#webSocketSessions = new Map<number, HostWebSocketSession>();
	#sessionIdsByWebSocket = new WeakMap<UniversalWebSocket, number>();
	#nextWebSocketSessionId = 1;
	#started = false;
	#disposed = false;

	constructor(config: DynamicActorHostRuntimeConfig) {
		this.#config = config;
		this.#runtimeDir = path.join(DYNAMIC_RUNTIME_ACTORS_ROOT, config.actorId);
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

		const sourceCode = await transpileDynamicSource(normalizedLoadResult.source);
		const sourcePath = path.join(this.#runtimeDir, "dynamic-source.cjs");
		await writeFile(sourcePath, sourceCode, "utf8");
		logger().debug({
			msg: "dynamic runtime source transpiled",
			actorId: this.#config.actorId,
			sourcePath,
		});

		const bootstrapPath = path.join(this.#runtimeDir, "dynamic-bootstrap.cjs");
		await writeFile(
			bootstrapPath,
			buildIsolateBootstrapSource({
				actorId: this.#config.actorId,
				actorName: this.#config.actorName,
				actorKey: this.#config.actorKey,
			}),
			"utf8",
		);
		logger().debug({
			msg: "dynamic runtime bootstrap written",
			actorId: this.#config.actorId,
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

		await this.#setIsolateBridge(ivm);
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
		if (!this.#fetchRef) {
			throw new Error("dynamic runtime is not started");
		}
		const input = await requestToEnvelope(request);
		const envelope = (await this.#fetchRef.apply(undefined, [input], {
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
		if (!this.#openWebSocketRef || !this.#sendWebSocketRef || !this.#closeWebSocketRef) {
			throw new Error("dynamic runtime websocket bridge is not started");
		}

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
			const gatewayIdBase64 = options.gatewayId
				? Buffer.from(new Uint8Array(options.gatewayId)).toString("base64")
				: undefined;
			const requestIdBase64 = options.requestId
				? Buffer.from(new Uint8Array(options.requestId)).toString("base64")
				: undefined;

			await this.#openWebSocketRef.apply(
				undefined,
				[
					{
						sessionId,
							path: pathValue,
							encoding,
							params,
							headers: options.headers,
							gatewayIdBase64,
							requestIdBase64,
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
		if (!this.#dispatchAlarmRef) return;
		await this.#dispatchAlarmRef.apply(undefined, [], {
			result: {
				copy: true,
				promise: true,
			},
		});
	}

	async getHibernatingWebSockets(): Promise<
		Array<DynamicHibernatingWebSocketMetadata>
	> {
		if (!this.#getHibernatingWebSocketsRef) {
			return [];
		}
		const entries = await this.#getHibernatingWebSocketsRef.apply(
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
		if (!this.#stopRef) return;
		try {
			await this.#stopRef.apply(undefined, [mode], {
				arguments: {
					copy: true,
				},
				result: {
					copy: true,
					promise: true,
				},
			});
		} catch (error) {
			logger().warn({
				msg: "failed to stop dynamic runtime actor",
				actorId: this.#config.actorId,
				mode,
				error: stringifyError(error),
			});
		}
	}

	async dispose(): Promise<void> {
		if (this.#disposed) return;
		this.#disposed = true;

		for (const session of this.#webSocketSessions.values()) {
			session.readyState = 3;
			session.websocket.triggerClose(1001, "dynamic.runtime.disposed", false);
		}
		this.#webSocketSessions.clear();

		if (this.#disposeRef) {
			try {
				await this.#disposeRef.apply(undefined, [], {
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

		this.#fetchRef = undefined;
		this.#dispatchAlarmRef = undefined;
		this.#stopRef = undefined;
		this.#openWebSocketRef = undefined;
		this.#sendWebSocketRef = undefined;
		this.#closeWebSocketRef = undefined;
		this.#getHibernatingWebSocketsRef = undefined;
		this.#disposeRef = undefined;
		this.#startSleepRef = undefined;
		this.#startDestroyRef = undefined;
		this.#started = false;
	}

	async #sendWebSocketMessage(
		sessionId: number,
		data: string | ArrayBufferLike | Blob | ArrayBufferView,
		rivetMessageIndex?: number,
	): Promise<void> {
		if (!this.#sendWebSocketRef) return;

		if (typeof data === "string") {
			await this.#sendWebSocketRef.apply(
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
		await this.#sendWebSocketRef.apply(
			undefined,
			[
					{
						sessionId,
						kind: "binary",
						dataBase64: Buffer.from(binary).toString("base64"),
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
		if (!this.#closeWebSocketRef) return;
		await this.#closeWebSocketRef.apply(
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

	async #setIsolateBridge(ivm: IsolatedVmModule): Promise<void> {
		if (!this.#context) {
			throw new Error("missing isolate context");
		}

		const context = this.#context;
		const makeRef = <T>(value: T): ReferenceLike<T> => {
			const ref = new ivm.Reference(value);
			this.#referenceHandles.push(ref as { release?: () => void });
			return ref;
		};

		const kvBatchPutRef = makeRef(
			async (
				actorId: string,
				entries: Array<[string, string]>,
			): Promise<void> => {
				const decodedEntries = entries.map(
					([key, value]) =>
						[
							new Uint8Array(Buffer.from(key, "base64")),
							new Uint8Array(Buffer.from(value, "base64")),
						] as [Uint8Array, Uint8Array],
				);
				await this.#config.actorDriver.kvBatchPut(actorId, decodedEntries);
			},
		);
		const kvBatchGetRef = makeRef(
			async (
				actorId: string,
				keys: string[],
			): Promise<string> => {
				const decodedKeys = keys.map(
					(key) => new Uint8Array(Buffer.from(key, "base64")),
				);
				const values = await this.#config.actorDriver.kvBatchGet(
					actorId,
					decodedKeys,
				);
				return JSON.stringify(values.map((value) =>
					value ? Buffer.from(value).toString("base64") : null,
				));
			},
		);
		const kvBatchDeleteRef = makeRef(
			async (actorId: string, keys: string[]): Promise<void> => {
				const decodedKeys = keys.map(
					(key) => new Uint8Array(Buffer.from(key, "base64")),
				);
				await this.#config.actorDriver.kvBatchDelete(actorId, decodedKeys);
			},
		);
		const kvListPrefixRef = makeRef(
			async (
				actorId: string,
				prefix: string,
			): Promise<string> => {
				const decodedPrefix = new Uint8Array(
					Buffer.from(prefix, "base64"),
				);
				const entries = await this.#config.actorDriver.kvListPrefix(
					actorId,
					decodedPrefix,
				);
				return JSON.stringify(entries.map(([key, value]) => [
					Buffer.from(key).toString("base64"),
					Buffer.from(value).toString("base64"),
				]));
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
		const startSleepRef = makeRef((actorId: string): void => {
			this.#config.actorDriver.startSleep?.(actorId);
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

		await context.global.set("__dynamicHostKvBatchPut", kvBatchPutRef);
		await context.global.set("__dynamicHostKvBatchGet", kvBatchGetRef);
		await context.global.set("__dynamicHostKvBatchDelete", kvBatchDeleteRef);
		await context.global.set("__dynamicHostKvListPrefix", kvListPrefixRef);
		await context.global.set("__dynamicHostSetAlarm", setAlarmRef);
		await context.global.set("__dynamicHostStartSleep", startSleepRef);
		await context.global.set("__dynamicHostStartDestroy", startDestroyRef);
		await context.global.set("__dynamicHostDispatch", dispatchRef);
		await context.global.set("__dynamicHostLog", logRef);
		this.#startSleepRef = startSleepRef;
		this.#startDestroyRef = startDestroyRef;
	}

	async #loadBootstrap(bootstrapPath: string): Promise<void> {
		if (!this.#context || !this.#nodeProcess) {
			throw new Error("missing isolate bootstrap dependencies");
		}

		logger().debug({
			msg: "dynamic runtime bootstrap compile begin",
			actorId: this.#config.actorId,
			bootstrapPath,
		});
		const bootstrapScript = await this.#nodeProcess.__unsafeIsoalte.compileScript(
			`
				const bootstrap = require(${JSON.stringify(bootstrapPath)});
				globalThis.__dynamicFetchEnvelope = bootstrap.dynamicFetchEnvelope;
				globalThis.__dynamicDispatchAlarmEnvelope = bootstrap.dynamicDispatchAlarmEnvelope;
				globalThis.__dynamicStopEnvelope = bootstrap.dynamicStopEnvelope;
					globalThis.__dynamicOpenWebSocketEnvelope = bootstrap.dynamicOpenWebSocketEnvelope;
					globalThis.__dynamicWebSocketSendEnvelope = bootstrap.dynamicWebSocketSendEnvelope;
					globalThis.__dynamicWebSocketCloseEnvelope = bootstrap.dynamicWebSocketCloseEnvelope;
					globalThis.__dynamicGetHibernatingWebSocketsEnvelope = bootstrap.dynamicGetHibernatingWebSocketsEnvelope;
					globalThis.__dynamicDisposeEnvelope = bootstrap.dynamicDisposeEnvelope;
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

		const getRef = async <T>(name: string): Promise<ReferenceLike<T>> => {
			const ref = (await this.#context!.global.get(name, {
				reference: true,
			})) as ReferenceLike<T>;
			this.#referenceHandles.push(ref as { release?: () => void });
			return ref;
		};

		this.#fetchRef = await getRef("__dynamicFetchEnvelope");
		this.#dispatchAlarmRef = await getRef("__dynamicDispatchAlarmEnvelope");
		this.#stopRef = await getRef("__dynamicStopEnvelope");
		this.#openWebSocketRef = await getRef("__dynamicOpenWebSocketEnvelope");
		this.#sendWebSocketRef = await getRef("__dynamicWebSocketSendEnvelope");
		this.#closeWebSocketRef = await getRef(
			"__dynamicWebSocketCloseEnvelope",
		);
		this.#getHibernatingWebSocketsRef = await getRef(
			"__dynamicGetHibernatingWebSocketsEnvelope",
		);
		this.#disposeRef = await getRef("__dynamicDisposeEnvelope");
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
		const bytes = payload.dataBase64
			? Buffer.from(payload.dataBase64, "base64")
			: Buffer.alloc(0);
		(session.websocket as any).triggerMessage(
			bytes,
			payload.rivetMessageIndex,
		);
	}
}

function normalizeLoadResult(loadResult: DynamicActorLoadResult): DynamicActorLoadResult {
	if (!loadResult || typeof loadResult.source !== "string") {
		throw new Error(
			"dynamic actor loader must return an object with a string `source` property",
		);
	}
	return loadResult;
}

async function requestToEnvelope(request: Request): Promise<FetchEnvelopeInput> {
	const headers: Record<string, string> = {};
	for (const [key, value] of request.headers.entries()) {
		headers[key] = value;
	}

	let bodyBase64: string | undefined;
	if (request.method !== "GET" && request.method !== "HEAD") {
		const body = await request.arrayBuffer();
		if (body.byteLength > 0) {
			bodyBase64 = Buffer.from(new Uint8Array(body)).toString("base64");
		}
	}

	return {
		url: request.url,
		method: request.method,
		headers,
		bodyBase64,
	};
}

function envelopeToResponse(envelope: FetchEnvelopeOutput): Response {
	return new Response(Buffer.from(envelope.bodyBase64, "base64"), {
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

async function ensureDynamicRuntimeNodeModules(): Promise<void> {
	if (!dynamicRuntimeSetupPromise) {
		dynamicRuntimeSetupPromise = (async () => {
			await mkdir(DYNAMIC_RUNTIME_ACTORS_ROOT, { recursive: true });
			const packageRoot = resolveRivetkitPackageRoot();
			const sourceNodeModules = await resolveDynamicNodeModulesSource(
				packageRoot,
			);
			const sourceDistEntry = path.join(packageRoot, "dist", "tsup", "mod.js");

			try {
				await stat(sourceNodeModules);
			} catch {
				throw new Error(
					`dynamic actor runtime requires node_modules at ${sourceNodeModules}`,
				);
			}

			try {
				await stat(sourceDistEntry);
			} catch {
				throw new Error(
					"dynamic actor runtime requires a built rivetkit package. Run `pnpm --filter rivetkit build` before using dynamicActor.",
				);
			}
			const sourceDistStat = await stat(sourceDistEntry);
			void sourceDistStat;

			// TODO: Temporary approach. Copy the workspace node_modules tree into a
			// shared dynamic runtime directory. Replace this with package allowlisting.
			await rm(DYNAMIC_RUNTIME_NODE_MODULES, {
				recursive: true,
				force: true,
			});
			await cp(sourceNodeModules, DYNAMIC_RUNTIME_NODE_MODULES, {
				recursive: true,
				dereference: true,
			});

			// Always materialize a local rivetkit package for dynamic actor module
			// resolution, even if the source node_modules tree does not include one.
			await materializeRuntimeRivetkitPackage(packageRoot);
			await materializeRuntimeDependencyClosure(packageRoot);
			await patchRuntimeOnChangePackageToCommonJs();
			await patchRuntimeNanoeventsPackageToCommonJs();
			await patchRuntimePRetryPackageToCommonJs();
			await patchRuntimeIsNetworkErrorPackageToCommonJs();
			await patchRuntimeGetPortPackageToCommonJs();
			await patchRuntimeTracesPackageToNoop();
		})();
	}
	return dynamicRuntimeSetupPromise;
}

async function materializeRuntimeRivetkitPackage(packageRoot: string): Promise<void> {
	const runtimeRivetkitRoot = path.join(DYNAMIC_RUNTIME_NODE_MODULES, "rivetkit");
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
	const chunkPath = path.join(
		runtimeRivetkitRoot,
		"dist",
		"tsup",
		"chunk-WGO75ZTR.cjs",
	);

	let sourceText: string;
	try {
		sourceText = await readFile(chunkPath, "utf8");
	} catch {
		return;
	}

	const patchedText = sourceText
		.replace(
			"createRequire.call(void 0, import.meta.url)",
			"createRequire.call(void 0, __filename)",
		)
		.replace(
			"url.fileURLToPath(import.meta.url)",
			"__filename",
		);

	if (patchedText !== sourceText) {
		await writeFile(chunkPath, patchedText, "utf8");
	}
}

async function materializeRuntimeDependencyClosure(
	packageRoot: string,
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
			await materializeRuntimePackageIfMissing(current.name, sourcePackageRoot);
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

async function patchRuntimeOnChangePackageToCommonJs(): Promise<void> {
	const packageSourceRoot = path.join(
		DYNAMIC_RUNTIME_NODE_MODULES,
		"@rivetkit",
		"on-change",
		"source",
	);
	try {
		await stat(packageSourceRoot);
	} catch {
		return;
	}

	// TODO: Temporary compatibility patch until sandboxed-node can safely load
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
}

async function patchRuntimeNanoeventsPackageToCommonJs(): Promise<void> {
	const packageSourceRoot = path.join(
		DYNAMIC_RUNTIME_NODE_MODULES,
		"nanoevents",
	);
	try {
		await stat(packageSourceRoot);
	} catch {
		return;
	}

	// TODO: Temporary compatibility patch until sandboxed-node can safely load
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
}

async function patchRuntimePRetryPackageToCommonJs(): Promise<void> {
	const packageSourceRoot = path.join(
		DYNAMIC_RUNTIME_NODE_MODULES,
		"p-retry",
	);
	try {
		await stat(packageSourceRoot);
	} catch {
		return;
	}

	// TODO: Temporary compatibility patch until sandboxed-node can safely load
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
}

async function patchRuntimeIsNetworkErrorPackageToCommonJs(): Promise<void> {
	const packageSourceRoot = path.join(
		DYNAMIC_RUNTIME_NODE_MODULES,
		"is-network-error",
	);
	try {
		await stat(packageSourceRoot);
	} catch {
		return;
	}

	// TODO: Temporary compatibility patch until sandboxed-node can safely load
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
}

async function patchRuntimeGetPortPackageToCommonJs(): Promise<void> {
	const packageSourceRoot = path.join(
		DYNAMIC_RUNTIME_NODE_MODULES,
		"get-port",
	);
	try {
		await stat(packageSourceRoot);
	} catch {
		return;
	}

	// TODO: Temporary compatibility patch until sandboxed-node can safely load
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
}

async function patchRuntimeTracesPackageToNoop(): Promise<void> {
	const tracesIndexPath = path.join(
		DYNAMIC_RUNTIME_NODE_MODULES,
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

	// TODO: Temporary compatibility patch until sandboxed-node can load the full
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
): Promise<void> {
	const runtimeDependencyRoot = path.join(
		DYNAMIC_RUNTIME_NODE_MODULES,
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

	return findNearestPackageRoot(resolvedPath);
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

async function resolveDynamicNodeModulesSource(
	packageRoot: string,
): Promise<string> {
	const dependencyNames = getRivetkitDependencyNames(packageRoot);
	let bestCandidatePath: string | undefined;
	let bestCoverageScore = -1;

	let current = packageRoot;
	while (true) {
		const candidate = path.join(current, "node_modules");
		try {
			await stat(candidate);
			const coverageScore = await scoreNodeModulesCoverage(
				candidate,
				dependencyNames,
			);
			if (coverageScore > bestCoverageScore) {
				bestCoverageScore = coverageScore;
				bestCandidatePath = candidate;
			}

			if (
				dependencyNames.length > 0 &&
				coverageScore >= dependencyNames.length
			) {
				return candidate;
			}
		} catch {
			// node_modules missing at this level, continue searching.
		}

		const parent = path.dirname(current);
		if (parent === current) {
			break;
		}
		current = parent;
	}

	return bestCandidatePath ?? path.join(packageRoot, "node_modules");
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

async function scoreNodeModulesCoverage(
	nodeModulesPath: string,
	dependencyNames: string[],
): Promise<number> {
	let score = 0;
	for (const dependencyName of dependencyNames) {
		try {
			await stat(path.join(nodeModulesPath, dependencyName));
			score += 1;
		} catch {
			// Dependency is not present in this node_modules candidate.
		}
	}
	return score;
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
			return secureExecRequire("isolated-vm") as IsolatedVmModule;
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

	try {
		return resolver.resolve("sandboxed-node");
	} catch {
		return path.join(
			process.env.HOME ?? "",
			"secure-exec-rivet/packages/sandboxed-node/dist/index.js",
		);
	}
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

function buildIsolateBootstrapSource(input: {
	actorId: string;
	actorName: string;
	actorKey: ActorKey;
}): string {
	return `
let setup;
let createActorRouter;
let routeWebSocket;
let InlineWebSocketAdapter;
let CONN_STATE_MANAGER_SYMBOL;
try {
	({
		setup,
		createActorRouter,
		routeWebSocket,
		InlineWebSocketAdapter,
		CONN_STATE_MANAGER_SYMBOL,
	} = require("rivetkit"));
} catch (error) {
	const details = error && error.stack ? error.stack : String(error);
	throw new Error(\`dynamic runtime failed to require rivetkit: \${details}\`);
}

let actorModule;
try {
	actorModule = require("./dynamic-source.cjs");
} catch (error) {
	const details = error && error.stack ? error.stack : String(error);
	throw new Error(\`dynamic runtime failed to require source module: \${details}\`);
}
const actorDefinition = actorModule.default ?? actorModule;
if (!actorDefinition || typeof actorDefinition.instantiate !== "function") {
	throw new Error("dynamic source module must default-export an ActorDefinition");
}

const actorId = ${JSON.stringify(input.actorId)};
const actorName = ${JSON.stringify(input.actorName)};
const actorKey = ${JSON.stringify(input.actorKey)};

const registry = setup({
	use: {
		[actorName]: actorDefinition,
	},
	serveManager: false,
	noWelcome: true,
	test: { enabled: false },
});
const config = registry.parseConfig();

let loadedActor = undefined;
let loadingActorPromise = undefined;
const webSocketSessions = new Map();

const inlineClient = new Proxy({}, {
	get() {
		throw new Error("dynamic actor sandbox does not support c.client() yet");
	},
});

function dynamicHostLog(level, message) {
	try {
		if (globalThis.__dynamicHostLog) {
			globalThis.__dynamicHostLog.applySync(undefined, [level, String(message)]);
		}
	} catch {}
}

function bridgeCall(ref, args) {
	return ref.applySyncPromise(undefined, args, {
		arguments: {
			copy: true,
		},
	});
}

function bridgeCallSync(ref, args) {
	return ref.applySync(undefined, args, {
		arguments: {
			copy: true,
		},
	});
}

function toBase64(buffer) {
	return Buffer.from(buffer).toString("base64");
}

function fromBase64(value) {
	return new Uint8Array(Buffer.from(value, "base64"));
}

function fromBase64ToArrayBuffer(value) {
	const buffer = Buffer.from(value, "base64");
	return buffer.buffer.slice(
		buffer.byteOffset,
		buffer.byteOffset + buffer.byteLength,
	);
}

function responseHeadersToEntries(headers) {
	if (!headers) {
		return [];
	}
	if (typeof headers.entries === "function") {
		return Array.from(headers.entries());
	}
	return Object.entries(headers).map(([key, value]) => [
		String(key),
		String(value),
	]);
}

async function responseBodyToBase64(response) {
	if (!response) {
		return "";
	}
	if (typeof response.arrayBuffer === "function") {
		const body = new Uint8Array(await response.arrayBuffer());
		return Buffer.from(body).toString("base64");
	}
	if (typeof response.text === "function") {
		const text = await response.text();
		return Buffer.from(text, "utf8").toString("base64");
	}

	const bodyValue = response.body;
	if (bodyValue === undefined || bodyValue === null) {
		return "";
	}
	if (typeof bodyValue.getReader === "function") {
		const reader = bodyValue.getReader();
		const chunks = [];
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
		return Buffer.from(merged).toString("base64");
	}
	if (typeof bodyValue === "string") {
		return Buffer.from(bodyValue, "utf8").toString("base64");
	}
	if (Array.isArray(bodyValue)) {
		return Buffer.from(bodyValue).toString("base64");
	}
	if (bodyValue instanceof Uint8Array) {
		return Buffer.from(bodyValue).toString("base64");
	}
	if (ArrayBuffer.isView(bodyValue)) {
		const view = new Uint8Array(
			bodyValue.buffer,
			bodyValue.byteOffset,
			bodyValue.byteLength,
		);
		return Buffer.from(view).toString("base64");
	}
	if (bodyValue instanceof ArrayBuffer) {
		return Buffer.from(new Uint8Array(bodyValue)).toString("base64");
	}
	return Buffer.from(String(bodyValue), "utf8").toString("base64");
}

async function loadActor(requestActorId) {
	if (requestActorId !== actorId) {
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
		const actor = actorDefinition.instantiate();
		try {
			await actor.start(actorDriver, inlineClient, actorId, actorName, actorKey, "unknown");
			loadedActor = actor;
		} catch (error) {
			dynamicHostLog(
				"warn",
				"actor.start failed: " +
					(error && error.stack ? error.stack : String(error)),
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

const actorDriver = {
	async loadActor(requestActorId) {
		return await loadActor(requestActorId);
	},
	getContext() {
		return {};
	},
	async kvBatchPut(actorIdValue, entries) {
		const encoded = entries.map(([key, value]) => [toBase64(key), toBase64(value)]);
		await bridgeCall(globalThis.__dynamicHostKvBatchPut, [actorIdValue, encoded]);
	},
	async kvBatchGet(actorIdValue, keys) {
		const encodedKeys = keys.map((key) => toBase64(key));
		const valuesJson = await bridgeCall(globalThis.__dynamicHostKvBatchGet, [actorIdValue, encodedKeys]);
		const values = JSON.parse(valuesJson);
		return values.map((value) => (value === null ? null : fromBase64(value)));
	},
	async kvBatchDelete(actorIdValue, keys) {
		const encodedKeys = keys.map((key) => toBase64(key));
		await bridgeCall(globalThis.__dynamicHostKvBatchDelete, [actorIdValue, encodedKeys]);
	},
	async kvListPrefix(actorIdValue, prefix) {
		const encodedPrefix = toBase64(prefix);
		const valuesJson = await bridgeCall(globalThis.__dynamicHostKvListPrefix, [actorIdValue, encodedPrefix]);
		const values = JSON.parse(valuesJson);
		return values.map(([key, value]) => [fromBase64(key), fromBase64(value)]);
	},
	async setAlarm(actor, timestamp) {
		await bridgeCall(globalThis.__dynamicHostSetAlarm, [actor.id, timestamp]);
	},
	startSleep(requestActorId) {
		bridgeCallSync(globalThis.__dynamicHostStartSleep, [requestActorId]);
	},
	startDestroy(requestActorId) {
		bridgeCallSync(globalThis.__dynamicHostStartDestroy, [requestActorId]);
	},
};

const actorRouter = createActorRouter(config, actorDriver, undefined, false);

async function dynamicHandleActionRequest(input, pathName) {
	const actor = await loadActor(actorId);
	const actionName = decodeURIComponent(pathName.slice("/action/".length));
	const encoding = input.headers?.["x-rivet-encoding"] || "json";
	if (encoding !== "json") {
		throw new Error(
			"dynamic action handler currently supports json encoding only",
		);
	}

	const bodyText = input.bodyBase64
		? Buffer.from(input.bodyBase64, "base64").toString("utf8")
		: "";
	let args = [];
	if (bodyText) {
		const parsed = JSON.parse(bodyText);
		if (Array.isArray(parsed?.args)) {
			args = parsed.args;
		}
	}

	const output = await actor.inspector.executeActionJson(actionName, args);
	return new Response(JSON.stringify({ output }), {
		status: 200,
		headers: {
			"content-type": "application/json",
		},
	});
}

async function dynamicFetchEnvelope(input) {
	const request = new Request(input.url, {
		method: input.method,
		headers: input.headers,
		body: input.bodyBase64 ? Buffer.from(input.bodyBase64, "base64") : undefined,
		duplex: "half",
	});
	const requestUrl = new URL(request.url);
	const response =
		request.method === "POST" && requestUrl.pathname.startsWith("/action/")
			? await dynamicHandleActionRequest(input, requestUrl.pathname)
			: await actorRouter.fetch(request, { actorId });
	const status = typeof response.status === "number" ? response.status : 200;
	const bodyBase64 = await responseBodyToBase64(response);
	if (status >= 500) {
		const preview = Buffer.from(bodyBase64, "base64").toString("utf8");
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
		bodyBase64,
	};
}

async function dynamicDispatchAlarmEnvelope() {
	const actor = await loadActor(actorId);
	await actor.onAlarm();
	return true;
}

async function dynamicStopEnvelope(mode) {
	if (!loadedActor) return true;
	await loadedActor.onStop(mode);
	loadedActor = undefined;
	return true;
}

async function dynamicOpenWebSocketEnvelope(input) {
	const headers = input.headers || {};
	const requestPath = input.path || "/connect";
	const pathOnly = requestPath.split("?")[0];
	const request = new Request(
		requestPath.startsWith("http") ? requestPath : \`http://actor\${requestPath}\`,
		{ method: "GET", headers },
	);
	const gatewayId = input.gatewayIdBase64
		? fromBase64ToArrayBuffer(input.gatewayIdBase64)
		: undefined;
	const requestId = input.requestIdBase64
		? fromBase64ToArrayBuffer(input.requestIdBase64)
		: undefined;
	const handler = await routeWebSocket(
		request,
		pathOnly,
		headers,
		config,
		actorDriver,
		actorId,
		input.encoding,
		input.params,
		gatewayId,
		requestId,
		!!input.isHibernatable,
		!!input.isRestoringHibernatable,
	);
	const adapter = new InlineWebSocketAdapter(handler);
	const ws = adapter.clientWebSocket;
	webSocketSessions.set(input.sessionId, { ws, adapter });

	ws.addEventListener("open", () => {
		bridgeCallSync(globalThis.__dynamicHostDispatch, [{
			type: "open",
			sessionId: input.sessionId,
		}]);
	});
	ws.addEventListener("message", (event) => {
		const data = event.data;
		if (typeof data === "string") {
			bridgeCallSync(globalThis.__dynamicHostDispatch, [{
				type: "message",
				sessionId: input.sessionId,
				kind: "text",
				text: data,
				rivetMessageIndex: event.rivetMessageIndex,
			}]);
			return;
		}
		if (data instanceof Blob) {
			data.arrayBuffer()
				.then((buffer) => {
					bridgeCallSync(globalThis.__dynamicHostDispatch, [{
						type: "message",
						sessionId: input.sessionId,
						kind: "binary",
						dataBase64: Buffer.from(new Uint8Array(buffer)).toString("base64"),
						rivetMessageIndex: event.rivetMessageIndex,
					}]);
				})
				.catch((error) => {
					bridgeCallSync(globalThis.__dynamicHostDispatch, [{
						type: "error",
						sessionId: input.sessionId,
						message: error instanceof Error ? error.message : String(error),
					}]);
				});
			return;
		}
		if (ArrayBuffer.isView(data)) {
			bridgeCallSync(globalThis.__dynamicHostDispatch, [{
				type: "message",
				sessionId: input.sessionId,
				kind: "binary",
				dataBase64: Buffer.from(new Uint8Array(data.buffer, data.byteOffset, data.byteLength)).toString("base64"),
				rivetMessageIndex: event.rivetMessageIndex,
			}]);
			return;
		}
		if (data instanceof ArrayBuffer) {
			bridgeCallSync(globalThis.__dynamicHostDispatch, [{
				type: "message",
				sessionId: input.sessionId,
				kind: "binary",
				dataBase64: Buffer.from(new Uint8Array(data)).toString("base64"),
				rivetMessageIndex: event.rivetMessageIndex,
			}]);
		}
	});
	ws.addEventListener("close", (event) => {
		webSocketSessions.delete(input.sessionId);
		bridgeCallSync(globalThis.__dynamicHostDispatch, [{
			type: "close",
			sessionId: input.sessionId,
			code: event.code,
			reason: event.reason,
			wasClean: event.wasClean,
		}]);
	});
	ws.addEventListener("error", (event) => {
		bridgeCallSync(globalThis.__dynamicHostDispatch, [{
			type: "error",
			sessionId: input.sessionId,
			message: event?.message || "dynamic websocket error",
		}]);
	});
	return true;
}

async function dynamicWebSocketSendEnvelope(input) {
	const session = webSocketSessions.get(input.sessionId);
	if (!session) return false;
	const payload =
		input.kind === "text"
			? input.text || ""
			: input.dataBase64
				? Buffer.from(input.dataBase64, "base64")
				: undefined;
	if (payload === undefined) {
		return false;
	}
	if (
		typeof session.adapter.dispatchClientMessageWithMetadata === "function"
	) {
		session.adapter.dispatchClientMessageWithMetadata(
			payload,
			input.rivetMessageIndex,
		);
		return true;
	}
	if (input.kind === "text") {
		session.ws.send(input.text || "");
		return true;
	}
	session.ws.send(Buffer.from(input.dataBase64, "base64"));
	return true;
}

async function dynamicWebSocketCloseEnvelope(input) {
	const session = webSocketSessions.get(input.sessionId);
	if (!session) return false;
	session.ws.close(input.code, input.reason);
	return true;
}

async function dynamicGetHibernatingWebSocketsEnvelope() {
	const actor = await loadActor(actorId);
	return Array.from(actor.conns.values())
		.map((conn) => {
			const connStateManager = conn[CONN_STATE_MANAGER_SYMBOL];
			const hibernatable = connStateManager?.hibernatableData;
			if (!hibernatable) return undefined;
			return {
				gatewayId: hibernatable.gatewayId,
				requestId: hibernatable.requestId,
				serverMessageIndex: hibernatable.serverMessageIndex,
				clientMessageIndex: hibernatable.clientMessageIndex,
				path: hibernatable.requestPath,
				headers: hibernatable.requestHeaders,
			};
		})
		.filter((entry) => entry !== undefined);
}

async function dynamicDisposeEnvelope() {
	for (const session of webSocketSessions.values()) {
		try {
			session.ws.close(1001, "dynamic.runtime.disposed");
		} catch {}
	}
	webSocketSessions.clear();
	return true;
}

module.exports = {
	dynamicFetchEnvelope,
	dynamicDispatchAlarmEnvelope,
	dynamicStopEnvelope,
	dynamicOpenWebSocketEnvelope,
	dynamicWebSocketSendEnvelope,
	dynamicWebSocketCloseEnvelope,
	dynamicGetHibernatingWebSocketsEnvelope,
	dynamicDisposeEnvelope,
};
`;
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

async function transpileDynamicSource(source: string): Promise<string> {
	return await transpileToCommonJs(source, "dynamic-source.ts");
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
