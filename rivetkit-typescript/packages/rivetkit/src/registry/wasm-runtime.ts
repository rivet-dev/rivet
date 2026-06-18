import type {
	ActorContext as WasmActorContext,
	ActorFactory as WasmActorFactory,
	CancellationToken as WasmCancellationToken,
	ConnHandle as WasmConnHandle,
	CoreRegistry as WasmCoreRegistry,
	WebSocketHandle as WasmWebSocketHandle,
} from "@rivetkit/rivetkit-wasm";
import { decodeBridgeRivetError } from "@/actor/errors";
import type {
	WasmRuntimeBindings,
	WasmRuntimeConfig,
	WasmRuntimeInitInput,
} from "./config";
import type {
	ActorContextHandle,
	ActorFactoryHandle,
	CancellationTokenHandle,
	ConnHandle,
	CoreRuntime,
	RegistryHandle,
	RuntimeActorConfig,
	RuntimeActorKeySegment,
	RuntimeBytes,
	RuntimeHttpRequest,
	RuntimeInspectorSnapshot,
	RuntimeKvEntry,
	RuntimeKvListOptions,
	RuntimeQueueEnqueueAndWaitOptions,
	RuntimeQueueInspectMessage,
	RuntimeQueueMessage,
	RuntimeQueueNextBatchOptions,
	RuntimeQueueTryNextBatchOptions,
	RuntimeQueueWaitOptions,
	RuntimeRequestSaveOpts,
	RuntimeServeConfig,
	RuntimeServerlessRequest,
	RuntimeServerlessResponseHead,
	RuntimeServerlessStreamCallback,
	RuntimeSqlBindParams,
	RuntimeSqlDatabase,
	RuntimeSqlExecResult,
	RuntimeSqlExecuteResult,
	RuntimeSqlQueryResult,
	RuntimeSqlRunResult,
	RuntimeStateDeltaPayload,
	RuntimeWebSocketEvent,
	WebSocketHandle,
} from "./runtime";
import { normalizeRuntimeSqlExecuteResult } from "./runtime";

type WasmBindings = WasmRuntimeBindings;
export type WasmInitInput = WasmRuntimeInitInput;
type WasmRuntimeLoadConfig = Pick<WasmRuntimeConfig, "bindings" | "initInput">;

function asWasmRegistry(handle: RegistryHandle): WasmCoreRegistry {
	return handle as unknown as WasmCoreRegistry;
}

function asWasmFactory(handle: ActorFactoryHandle): WasmActorFactory {
	return handle as unknown as WasmActorFactory;
}

function asWasmActorContext(handle: ActorContextHandle): WasmActorContext {
	return handle as unknown as WasmActorContext;
}

function asWasmConn(handle: ConnHandle): WasmConnHandle {
	return handle as unknown as WasmConnHandle;
}

function asWasmWebSocket(handle: WebSocketHandle): WasmWebSocketHandle {
	return handle as unknown as WasmWebSocketHandle;
}

function asWasmCancellationToken(
	handle: CancellationTokenHandle,
): WasmCancellationToken {
	return handle as unknown as WasmCancellationToken;
}

function asRegistryHandle(handle: WasmCoreRegistry): RegistryHandle {
	return handle as unknown as RegistryHandle;
}

function asActorFactoryHandle(handle: WasmActorFactory): ActorFactoryHandle {
	return handle as unknown as ActorFactoryHandle;
}

function toBytes(value: RuntimeBytes | null | undefined): RuntimeBytes {
	return value ?? new Uint8Array(0);
}

function optionalBytes(
	value: RuntimeBytes | null | undefined,
): RuntimeBytes | null {
	if (value === null || value === undefined) {
		return null;
	}
	return toBytes(value);
}

function optionalWasmNumber(
	value: number | bigint | null | undefined,
): number | null | undefined {
	if (value === null || value === undefined) {
		return value;
	}
	return typeof value === "bigint" ? Number(value) : value;
}

function wasmNumber(value: number | bigint): number {
	return typeof value === "bigint" ? Number(value) : value;
}

function normalizeKvEntry(entry: RuntimeKvEntry): RuntimeKvEntry {
	return {
		key: toBytes(entry.key),
		value: toBytes(entry.value),
	};
}

function normalizeQueueMessage(
	message: RuntimeQueueMessage,
): RuntimeQueueMessage {
	return {
		id: () => message.id(),
		name: () => message.name(),
		body: () => toBytes(message.body()),
		createdAt: () => message.createdAt(),
		isCompletable: () => message.isCompletable(),
		complete: async (response?: RuntimeBytes | undefined | null) => {
			await callWasm(() => message.complete(response));
		},
	};
}

function normalizeWasmBridgeError(error: unknown): unknown {
	if (typeof error === "string") {
		return decodeBridgeRivetError(error) ?? error;
	}

	if (error instanceof Error) {
		const bridged = decodeBridgeRivetError(error.message);
		if (bridged) {
			return bridged;
		}
	}

	if (
		typeof error === "object" &&
		error !== null &&
		"reason" in error &&
		typeof error.reason === "string"
	) {
		const bridged = decodeBridgeRivetError(error.reason);
		if (bridged) {
			return bridged;
		}
	}

	return error;
}

async function callWasm<T>(invoke: () => Promise<T>): Promise<T> {
	try {
		return await invoke();
	} catch (error) {
		throw normalizeWasmBridgeError(error);
	}
}

function callWasmSync<T>(invoke: () => T): T {
	try {
		return invoke();
	} catch (error) {
		throw normalizeWasmBridgeError(error);
	}
}

export class WasmCoreRuntime implements CoreRuntime {
	readonly kind = "wasm";

	#bindings: WasmBindings;
	#sql = new WeakMap<WasmActorContext, RuntimeSqlDatabase>();

	constructor(bindings: WasmBindings) {
		this.#bindings = bindings;
	}

	#actorSql(ctx: ActorContextHandle): RuntimeSqlDatabase {
		const wasmCtx = asWasmActorContext(ctx);
		let database = this.#sql.get(wasmCtx);
		if (!database) {
			database = callWasmSync(
				() => wasmCtx.sql() as unknown as RuntimeSqlDatabase,
			);
			this.#sql.set(wasmCtx, database);
		}
		return database;
	}

	createRegistry(): RegistryHandle {
		return callWasmSync(() =>
			asRegistryHandle(new this.#bindings.CoreRegistry()),
		);
	}

	registerActor(
		registry: RegistryHandle,
		name: string,
		factory: ActorFactoryHandle,
	): void {
		callWasmSync(() =>
			asWasmRegistry(registry).register(name, asWasmFactory(factory)),
		);
	}

	async serveRegistry(
		registry: RegistryHandle,
		config: RuntimeServeConfig,
	): Promise<void> {
		await callWasm(() => asWasmRegistry(registry).serve(config));
	}

	async shutdownRegistry(registry: RegistryHandle): Promise<void> {
		await callWasm(() => asWasmRegistry(registry).shutdown());
	}

	async registryDiagnostics(): Promise<{ mode: string; envoyActiveActorCount: null }> {
		return { mode: "wasm", envoyActiveActorCount: null };
	}

	async handleServerlessRequest(
		registry: RegistryHandle,
		req: RuntimeServerlessRequest,
		onStreamEvent: RuntimeServerlessStreamCallback,
		cancelToken: CancellationTokenHandle,
		config: RuntimeServeConfig,
	): Promise<RuntimeServerlessResponseHead> {
		return (await callWasm(() =>
			asWasmRegistry(registry).handleServerlessRequest(
				req,
				onStreamEvent as unknown as Function,
				asWasmCancellationToken(cancelToken),
				config,
			),
		)) as RuntimeServerlessResponseHead;
	}

	createActorFactory(
		callbacks: object,
		config?: RuntimeActorConfig | undefined | null,
	): ActorFactoryHandle {
		return callWasmSync(() =>
			asActorFactoryHandle(
				new this.#bindings.ActorFactory(callbacks, config),
			),
		);
	}

	createCancellationToken(): CancellationTokenHandle {
		return callWasmSync(
			() =>
				new this.#bindings.CancellationToken() as unknown as CancellationTokenHandle,
		);
	}

	cancellationTokenAborted(token: CancellationTokenHandle): boolean {
		return callWasmSync(() => asWasmCancellationToken(token).aborted());
	}

	cancelCancellationToken(token: CancellationTokenHandle): void {
		callWasmSync(() => asWasmCancellationToken(token).cancel());
	}

	onCancellationTokenCancelled(
		token: CancellationTokenHandle,
		callback: (...args: unknown[]) => unknown,
	): void {
		callWasmSync(() =>
			asWasmCancellationToken(token).onCancelled(callback),
		);
	}

	actorState(ctx: ActorContextHandle): RuntimeBytes {
		return toBytes(callWasmSync(() => asWasmActorContext(ctx).state()));
	}

	actorBeginOnStateChange(ctx: ActorContextHandle): void {
		callWasmSync(() => asWasmActorContext(ctx).beginOnStateChange());
	}

	actorEndOnStateChange(ctx: ActorContextHandle): void {
		callWasmSync(() => asWasmActorContext(ctx).endOnStateChange());
	}

	actorSetAlarm(
		ctx: ActorContextHandle,
		timestampMs?: number | bigint | undefined | null,
	): void {
		callWasmSync(() =>
			asWasmActorContext(ctx).setAlarm(optionalWasmNumber(timestampMs)),
		);
	}

	actorRequestSave(
		ctx: ActorContextHandle,
		opts?: RuntimeRequestSaveOpts | undefined | null,
	): void {
		callWasmSync(() => asWasmActorContext(ctx).requestSave(opts));
	}

	async actorRequestSaveAndWait(
		ctx: ActorContextHandle,
		opts?: RuntimeRequestSaveOpts | undefined | null,
	): Promise<void> {
		await callWasm(() => asWasmActorContext(ctx).requestSaveAndWait(opts));
	}

	actorInspectorSnapshot(ctx: ActorContextHandle): RuntimeInspectorSnapshot {
		return callWasmSync(
			() =>
				asWasmActorContext(ctx).inspectorSnapshot() as RuntimeInspectorSnapshot,
		);
	}

	actorDecodeInspectorRequest(
		ctx: ActorContextHandle,
		bytes: RuntimeBytes,
		advertisedVersion: number,
	): RuntimeBytes {
		return toBytes(
			callWasmSync(() =>
				// @ts-expect-error WASM parity P1.1: implement decodeInspectorRequest on WasmActorContext
				asWasmActorContext(ctx).decodeInspectorRequest(
					bytes,
					advertisedVersion,
				),
			),
		);
	}

	actorEncodeInspectorResponse(
		ctx: ActorContextHandle,
		bytes: RuntimeBytes,
		targetVersion: number,
	): RuntimeBytes {
		return toBytes(
			callWasmSync(() =>
				// @ts-expect-error WASM parity P1.1: implement encodeInspectorResponse on WasmActorContext
				asWasmActorContext(ctx).encodeInspectorResponse(bytes, targetVersion),
			),
		);
	}

	async actorVerifyInspectorAuth(
		ctx: ActorContextHandle,
		bearerToken?: string | undefined | null,
	): Promise<void> {
		await callWasm(() =>
			asWasmActorContext(ctx).verifyInspectorAuth(bearerToken),
		);
	}

	actorQueueHibernationRemoval(
		ctx: ActorContextHandle,
		connId: string,
	): void {
		callWasmSync(() =>
			// @ts-expect-error WASM parity P1.2: implement queueHibernationRemoval on WasmActorContext
			asWasmActorContext(ctx).queueHibernationRemoval(connId),
		);
	}

	actorTakePendingHibernationChanges(ctx: ActorContextHandle): string[] {
		return callWasmSync(
			() =>
				asWasmActorContext(ctx).takePendingHibernationChanges() as string[],
		);
	}

	actorDirtyHibernatableConns(ctx: ActorContextHandle): ConnHandle[] {
		return callWasmSync(
			() =>
				asWasmActorContext(ctx).dirtyHibernatableConns() as unknown as ConnHandle[],
		);
	}

	async actorSaveState(
		ctx: ActorContextHandle,
		payload: RuntimeStateDeltaPayload,
	): Promise<void> {
		await callWasm(() => asWasmActorContext(ctx).saveState(payload));
	}

	actorId(ctx: ActorContextHandle): string {
		return callWasmSync(() => asWasmActorContext(ctx).actorId());
	}

	actorName(ctx: ActorContextHandle): string {
		return callWasmSync(() => asWasmActorContext(ctx).name());
	}

	actorKey(ctx: ActorContextHandle): RuntimeActorKeySegment[] {
		return callWasmSync(
			() => asWasmActorContext(ctx).key() as RuntimeActorKeySegment[],
		);
	}

	actorRegion(ctx: ActorContextHandle): string {
		return callWasmSync(() => asWasmActorContext(ctx).region());
	}

	actorSleep(ctx: ActorContextHandle): void {
		callWasmSync(() => asWasmActorContext(ctx).sleep());
	}

	actorDestroy(ctx: ActorContextHandle): void {
		callWasmSync(() => asWasmActorContext(ctx).destroy());
	}

	actorAbortSignal(ctx: ActorContextHandle): AbortSignal {
		return callWasmSync(
			() => asWasmActorContext(ctx).abortSignal() as AbortSignal,
		);
	}

	actorConns(ctx: ActorContextHandle): ConnHandle[] {
		return callWasmSync(
			() => asWasmActorContext(ctx).conns() as unknown as ConnHandle[],
		);
	}

	async actorConnectConn(
		ctx: ActorContextHandle,
		params: RuntimeBytes,
		request?: RuntimeHttpRequest | undefined | null,
	): Promise<ConnHandle> {
		return (await callWasm(() =>
			asWasmActorContext(ctx).connectConn(params, request),
		)) as unknown as ConnHandle;
	}

	actorBroadcast(
		ctx: ActorContextHandle,
		name: string,
		args: RuntimeBytes,
	): void {
		callWasmSync(() => asWasmActorContext(ctx).broadcast(name, args));
	}

	actorWaitUntil(ctx: ActorContextHandle, promise: Promise<unknown>): void {
		callWasmSync(() =>
			asWasmActorContext(ctx).waitUntil(promise as Promise<unknown>),
		);
	}

	actorKeepAwake(ctx: ActorContextHandle, promise: Promise<unknown>): void {
		const wasmCtx = asWasmActorContext(ctx);
		const regionId = callWasmSync(() => wasmCtx.beginKeepAwake());
		const trackedPromise = Promise.resolve(promise)
			.finally(() => {
				callWasmSync(() => wasmCtx.endKeepAwake(regionId));
			})
			.then(() => null);
		callWasmSync(() => wasmCtx.registerTask(trackedPromise));
	}

	actorBeginKeepAwake(ctx: ActorContextHandle): number {
		return callWasmSync(() => asWasmActorContext(ctx).beginKeepAwake());
	}

	actorEndKeepAwake(ctx: ActorContextHandle, regionId: number): void {
		callWasmSync(() => asWasmActorContext(ctx).endKeepAwake(regionId));
	}

	actorRegisterTask(
		ctx: ActorContextHandle,
		promise: Promise<unknown>,
	): void {
		callWasmSync(() => asWasmActorContext(ctx).registerTask(promise));
	}

	actorRuntimeState(ctx: ActorContextHandle): object {
		return callWasmSync(
			() => asWasmActorContext(ctx).runtimeState() as object,
		);
	}

	actorClearRuntimeState(ctx: ActorContextHandle): void {
		const runtimeState = this.actorRuntimeState(ctx);
		for (const key of Object.keys(runtimeState)) {
			delete (runtimeState as Record<string, unknown>)[key];
		}
	}

	actorRestartRunHandler(ctx: ActorContextHandle): void {
		callWasmSync(() => asWasmActorContext(ctx).restartRunHandler());
	}

	actorBeginWebsocketCallback(ctx: ActorContextHandle): number {
		return callWasmSync(() =>
			asWasmActorContext(ctx).beginWebsocketCallback(),
		);
	}

	actorEndWebsocketCallback(ctx: ActorContextHandle, regionId: number): void {
		callWasmSync(() =>
			asWasmActorContext(ctx).endWebsocketCallback(regionId),
		);
	}

	async actorKvGet(
		ctx: ActorContextHandle,
		key: RuntimeBytes,
	): Promise<RuntimeBytes | null> {
		const kv = callWasmSync(() => asWasmActorContext(ctx).kv());
		return optionalBytes(await callWasm(() => kv.get(key)));
	}

	async actorKvPut(
		ctx: ActorContextHandle,
		key: RuntimeBytes,
		value: RuntimeBytes,
	): Promise<void> {
		const kv = callWasmSync(() => asWasmActorContext(ctx).kv());
		await callWasm(() => kv.put(key, value));
	}

	async actorKvDelete(
		ctx: ActorContextHandle,
		key: RuntimeBytes,
	): Promise<void> {
		const kv = callWasmSync(() => asWasmActorContext(ctx).kv());
		await callWasm(() => kv.delete(key));
	}

	async actorKvDeleteRange(
		ctx: ActorContextHandle,
		start: RuntimeBytes,
		end: RuntimeBytes,
	): Promise<void> {
		const kv = callWasmSync(() => asWasmActorContext(ctx).kv());
		await callWasm(() => kv.deleteRange(start, end));
	}

	async actorKvListPrefix(
		ctx: ActorContextHandle,
		prefix: RuntimeBytes,
		options?: RuntimeKvListOptions | undefined | null,
	): Promise<RuntimeKvEntry[]> {
		const kv = callWasmSync(() => asWasmActorContext(ctx).kv());
		const entries = (await callWasm(() =>
			kv.listPrefix(prefix, options),
		)) as RuntimeKvEntry[];
		return entries.map(normalizeKvEntry);
	}

	async actorKvListRange(
		ctx: ActorContextHandle,
		start: RuntimeBytes,
		end: RuntimeBytes,
		options?: RuntimeKvListOptions | undefined | null,
	): Promise<RuntimeKvEntry[]> {
		const kv = callWasmSync(() => asWasmActorContext(ctx).kv());
		const entries = (await callWasm(() =>
			kv.listRange(start, end, options),
		)) as RuntimeKvEntry[];
		return entries.map(normalizeKvEntry);
	}

	async actorKvBatchGet(
		ctx: ActorContextHandle,
		keys: RuntimeBytes[],
	): Promise<Array<RuntimeBytes | undefined | null>> {
		const kv = callWasmSync(() => asWasmActorContext(ctx).kv());
		const values = (await callWasm(() => kv.batchGet(keys))) as Array<
			RuntimeBytes | Uint8Array | null | undefined
		>;
		return values.map((value) =>
			value === undefined ? undefined : optionalBytes(value),
		);
	}

	async actorKvBatchPut(
		ctx: ActorContextHandle,
		entries: RuntimeKvEntry[],
	): Promise<void> {
		const kv = callWasmSync(() => asWasmActorContext(ctx).kv());
		await callWasm(() => kv.batchPut(entries));
	}

	async actorKvBatchDelete(
		ctx: ActorContextHandle,
		keys: RuntimeBytes[],
	): Promise<void> {
		const kv = callWasmSync(() => asWasmActorContext(ctx).kv());
		await callWasm(() => kv.batchDelete(keys));
	}

	async actorSqlExec(
		ctx: ActorContextHandle,
		sql: string,
	): Promise<RuntimeSqlExecResult> {
		return await callWasm(() => this.#actorSql(ctx).exec(sql));
	}

	async actorSqlExecute(
		ctx: ActorContextHandle,
		sql: string,
		params?: RuntimeSqlBindParams,
	): Promise<RuntimeSqlExecuteResult> {
		const result = await callWasm(() =>
			this.#actorSql(ctx).execute(sql, params),
		);
		return normalizeRuntimeSqlExecuteResult(result);
	}

	async actorSqlQuery(
		ctx: ActorContextHandle,
		sql: string,
		params?: RuntimeSqlBindParams,
	): Promise<RuntimeSqlQueryResult> {
		return await callWasm(() => this.#actorSql(ctx).query(sql, params));
	}

	async actorSqlRun(
		ctx: ActorContextHandle,
		sql: string,
		params?: RuntimeSqlBindParams,
	): Promise<RuntimeSqlRunResult> {
		return await callWasm(() => this.#actorSql(ctx).run(sql, params));
	}

	actorSqlMetrics(ctx: ActorContextHandle) {
		return this.#actorSql(ctx).metrics?.() ?? null;
	}

	actorSqlTakeLastKvError(ctx: ActorContextHandle): string | null {
		return this.#actorSql(ctx).takeLastKvError?.() ?? null;
	}

	async actorSqlClose(ctx: ActorContextHandle): Promise<void> {
		const wasmCtx = asWasmActorContext(ctx);
		const database = this.#sql.get(wasmCtx);
		if (!database) {
			return;
		}

		this.#sql.delete(wasmCtx);
		await callWasm(() => database.close());
	}

	async actorQueueSend(
		ctx: ActorContextHandle,
		name: string,
		body: RuntimeBytes,
	): Promise<RuntimeQueueMessage> {
		const queue = callWasmSync(() => asWasmActorContext(ctx).queue());
		return normalizeQueueMessage(
			(await callWasm(() => queue.send(name, body))) as RuntimeQueueMessage,
		);
	}

	async actorQueueNextBatch(
		ctx: ActorContextHandle,
		options?: RuntimeQueueNextBatchOptions | undefined | null,
		signal?: CancellationTokenHandle | undefined | null,
	): Promise<RuntimeQueueMessage[]> {
		const queue = callWasmSync(() => asWasmActorContext(ctx).queue());
		const messages = (await callWasm(() =>
			queue.nextBatch(options, signal ? asWasmCancellationToken(signal) : null),
		)) as RuntimeQueueMessage[];
		return messages.map(normalizeQueueMessage);
	}

	async actorQueueWaitForNames(
		ctx: ActorContextHandle,
		names: string[],
		options?: RuntimeQueueWaitOptions | undefined | null,
		signal?: CancellationTokenHandle | undefined | null,
	): Promise<RuntimeQueueMessage> {
		const queue = callWasmSync(() => asWasmActorContext(ctx).queue());
		return normalizeQueueMessage(
			(await callWasm(() =>
				queue.waitForNames(
					names,
					options,
					signal ? asWasmCancellationToken(signal) : null,
				),
			)) as unknown as RuntimeQueueMessage,
		);
	}

	async actorQueueWaitForNamesAvailable(
		ctx: ActorContextHandle,
		names: string[],
		options?: RuntimeQueueWaitOptions | undefined | null,
	): Promise<void> {
		const queue = callWasmSync(() => asWasmActorContext(ctx).queue());
		await callWasm(() => queue.waitForNamesAvailable(names, options));
	}

	async actorQueueEnqueueAndWait(
		ctx: ActorContextHandle,
		name: string,
		body: RuntimeBytes,
		options?: RuntimeQueueEnqueueAndWaitOptions | undefined | null,
		signal?: CancellationTokenHandle | undefined | null,
	): Promise<RuntimeBytes | null> {
		const queue = callWasmSync(() => asWasmActorContext(ctx).queue());
		return optionalBytes(
			await callWasm(() =>
				queue.enqueueAndWait(
					name,
					body,
					options,
					signal ? asWasmCancellationToken(signal) : null,
				),
			),
		);
	}

	actorQueueTryNextBatch(
		ctx: ActorContextHandle,
		options?: RuntimeQueueTryNextBatchOptions | undefined | null,
	): RuntimeQueueMessage[] {
		const queue = callWasmSync(() => asWasmActorContext(ctx).queue());
		return (
			callWasmSync(() => queue.tryNextBatch(options)) as RuntimeQueueMessage[]
		).map(normalizeQueueMessage);
	}

	actorQueueMaxSize(ctx: ActorContextHandle): number {
		const queue = callWasmSync(() => asWasmActorContext(ctx).queue());
		return callWasmSync(() => queue.maxSize());
	}

	async actorQueueInspectMessages(
		ctx: ActorContextHandle,
	): Promise<RuntimeQueueInspectMessage[]> {
		const queue = callWasmSync(() => asWasmActorContext(ctx).queue());
		return (await callWasm(() =>
			queue.inspectMessages(),
		)) as RuntimeQueueInspectMessage[];
	}

	actorScheduleAfter(
		ctx: ActorContextHandle,
		durationMs: number | bigint,
		actionName: string,
		args: RuntimeBytes,
	): void {
		const schedule = callWasmSync(() => asWasmActorContext(ctx).schedule());
		callWasmSync(() =>
			schedule.after(wasmNumber(durationMs), actionName, args),
		);
	}

	actorScheduleAt(
		ctx: ActorContextHandle,
		timestampMs: number | bigint,
		actionName: string,
		args: RuntimeBytes,
	): void {
		const schedule = callWasmSync(() => asWasmActorContext(ctx).schedule());
		callWasmSync(() =>
			schedule.at(wasmNumber(timestampMs), actionName, args),
		);
	}

	connId(conn: ConnHandle): string {
		return callWasmSync(() => asWasmConn(conn).id());
	}

	connParams(conn: ConnHandle): RuntimeBytes {
		return toBytes(callWasmSync(() => asWasmConn(conn).params()));
	}

	connState(conn: ConnHandle): RuntimeBytes {
		return toBytes(callWasmSync(() => asWasmConn(conn).state()));
	}

	connSetState(conn: ConnHandle, state: RuntimeBytes): void {
		callWasmSync(() => asWasmConn(conn).setState(state));
	}

	connIsHibernatable(conn: ConnHandle): boolean {
		return callWasmSync(() => asWasmConn(conn).isHibernatable());
	}

	connSend(conn: ConnHandle, name: string, args: RuntimeBytes): void {
		callWasmSync(() => asWasmConn(conn).send(name, args));
	}

	async connDisconnect(
		conn: ConnHandle,
		reason?: string | undefined | null,
	): Promise<void> {
		await callWasm(() => asWasmConn(conn).disconnect(reason));
	}

	webSocketSend(
		ws: WebSocketHandle,
		data: RuntimeBytes,
		binary: boolean,
	): void {
		callWasmSync(() => asWasmWebSocket(ws).send(data, binary));
	}

	async webSocketClose(
		ws: WebSocketHandle,
		code?: number | undefined | null,
		reason?: string | undefined | null,
	): Promise<void> {
		await callWasm(() => asWasmWebSocket(ws).close(code, reason));
	}

	webSocketSetEventCallback(
		ws: WebSocketHandle,
		callback: (event: RuntimeWebSocketEvent) => void,
	): void {
		callWasmSync(() =>
			asWasmWebSocket(ws).setEventCallback(
				((event: RuntimeWebSocketEvent) => {
					if (event.kind === "message" && event.binary) {
						callback({
							...event,
							data: toBytes(event.data as RuntimeBytes | Uint8Array),
						});
						return;
					}
					callback(event);
				}) as unknown as Function,
			),
		);
	}
}

export type { WasmBindings };

export async function loadWasmRuntime(config?: WasmRuntimeLoadConfig): Promise<{
	bindings: WasmBindings;
	runtime: WasmCoreRuntime;
}> {
	const bindings =
		config?.bindings ??
		(await import(["@rivetkit", "rivetkit-wasm"].join("/")));
	await bindings.default(config?.initInput);
	return {
		bindings,
		runtime: new WasmCoreRuntime(bindings),
	};
}
