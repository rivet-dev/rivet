import * as cbor from "cbor-x";
import { getRunFunction } from "@/actor/config";
import type { AnyActorDefinition } from "@/actor/definition";
import type { RegistryConfig } from "@/registry/config";
import { logger } from "./log";

import type {
	ActorContext as NativeActorContext,
	ConnHandle as NativeConnHandle,
	CoreRegistry as NativeCoreRegistry,
	JsActorConfig,
	JsFactoryInitResult,
	JsHttpResponse,
	JsServeConfig,
	NapiActorFactory as NativeActorFactory,
	Queue as NativeQueue,
	QueueMessage as NativeQueueMessage,
	Schedule as NativeSchedule,
	WebSocket as NativeWebSocket,
} from "@rivetkit/rivetkit-napi";

type NativeBindings = typeof import("@rivetkit/rivetkit-napi");

async function loadNativeBindings(): Promise<NativeBindings> {
	return import(["@rivetkit", "rivetkit-napi"].join("/"));
}

async function loadEngineCli(): Promise<typeof import("@rivetkit/engine-cli")> {
	return import(["@rivetkit", "engine-cli"].join("/"));
}

function decodeValue<T>(value?: Buffer | Uint8Array | null): T {
	if (!value || value.length === 0) {
		return undefined as T;
	}

	return cbor.decode(Buffer.from(value)) as T;
}

function encodeValue(value: unknown): Buffer {
	return Buffer.from(cbor.encode(value));
}

function decodeArgs(value?: Buffer | Uint8Array | null): unknown[] {
	const decoded = decodeValue<unknown>(value);
	return Array.isArray(decoded) ? decoded : decoded === undefined ? [] : [decoded];
}

function buildRequest(init: {
	method: string;
	uri: string;
	headers?: Record<string, string>;
	body?: Buffer;
}): Request {
	const url = init.uri.startsWith("http")
		? init.uri
		: new URL(init.uri, "http://127.0.0.1").toString();
	const body = init.body && init.body.length > 0 ? init.body : undefined;
	return new Request(url, {
		method: init.method,
		headers: init.headers,
		body,
	});
}

async function toJsHttpResponse(response: Response): Promise<JsHttpResponse> {
	const headers = Object.fromEntries(response.headers.entries());
	const body = Buffer.from(await response.arrayBuffer());
	return {
		status: response.status,
		headers,
		body,
	};
}

function toActorKey(
	segments: Array<{ kind: string; stringValue?: string; numberValue?: number }>,
): Array<string | number> {
	return segments.map((segment) =>
		segment.kind === "number"
			? (segment.numberValue ?? 0)
			: (segment.stringValue ?? ""),
	);
}

class NativeConnAdapter {
	#conn: NativeConnHandle;

	constructor(conn: NativeConnHandle) {
		this.#conn = conn;
	}

	get id(): string {
		return this.#conn.id();
	}

	get params(): unknown {
		return decodeValue(this.#conn.params());
	}

	get state(): unknown {
		return decodeValue(this.#conn.state());
	}

	set state(value: unknown) {
		this.#conn.setState(encodeValue(value));
	}

	get isHibernatable(): boolean {
		return this.#conn.isHibernatable();
	}

	send(name: string, ...args: unknown[]): void {
		this.#conn.send(name, encodeValue(args));
	}

	async disconnect(reason?: string): Promise<void> {
		await this.#conn.disconnect(reason);
	}
}

class NativeScheduleAdapter {
	#schedule: NativeSchedule;

	constructor(schedule: NativeSchedule) {
		this.#schedule = schedule;
	}

	async after(duration: number, action: string, ...args: unknown[]): Promise<void> {
		this.#schedule.after(duration, action, encodeValue(args));
	}

	async at(timestamp: number, action: string, ...args: unknown[]): Promise<void> {
		this.#schedule.at(timestamp, action, encodeValue(args));
	}
}

function wrapQueueMessage(message: NativeQueueMessage) {
	return {
		id: Number(message.id()),
		name: message.name(),
		body: decodeValue(message.body()),
		createdAt: message.createdAt(),
		complete: message.isCompletable()
			? async (response?: unknown) =>
					await message.complete(
						response === undefined ? undefined : encodeValue(response),
					)
			: undefined,
	};
}

class NativeQueueAdapter {
	#queue: NativeQueue;

	constructor(queue: NativeQueue) {
		this.#queue = queue;
	}

	async send(name: string, body: unknown) {
		return wrapQueueMessage(await this.#queue.send(name, encodeValue(body)));
	}

	async next(options?: {
		names?: readonly string[];
		timeout?: number;
		completable?: boolean;
	}) {
		const message = await this.#queue.next({
			names: options?.names ? [...options.names] : undefined,
			timeoutMs: options?.timeout,
			completable: options?.completable,
		});
		return message ? wrapQueueMessage(message) : undefined;
	}

	async nextBatch(options?: {
		names?: readonly string[];
		count?: number;
		timeout?: number;
		completable?: boolean;
	}) {
		const messages = await this.#queue.nextBatch({
			names: options?.names ? [...options.names] : undefined,
			count: options?.count,
			timeoutMs: options?.timeout,
			completable: options?.completable,
		});
		return messages.map(wrapQueueMessage);
	}

	async tryNext(options?: {
		names?: readonly string[];
		completable?: boolean;
	}) {
		const message = this.#queue.tryNext({
			names: options?.names ? [...options.names] : undefined,
			completable: options?.completable,
		});
		return message ? wrapQueueMessage(message) : undefined;
	}

	async tryNextBatch(options?: {
		names?: readonly string[];
		count?: number;
		completable?: boolean;
	}) {
		const messages = this.#queue.tryNextBatch({
			names: options?.names ? [...options.names] : undefined,
			count: options?.count,
			completable: options?.completable,
		});
		return messages.map(wrapQueueMessage);
	}
}

class NativeWebSocketAdapter {
	#ws: NativeWebSocket;

	constructor(ws: NativeWebSocket) {
		this.#ws = ws;
	}

	send(data: string | ArrayBuffer | ArrayBufferView): void {
		if (typeof data === "string") {
			this.#ws.send(Buffer.from(data), false);
			return;
		}

		const buffer = ArrayBuffer.isView(data)
			? Buffer.from(data.buffer, data.byteOffset, data.byteLength)
			: Buffer.from(data);
		this.#ws.send(buffer, true);
	}

	close(code?: number, reason?: string): void {
		this.#ws.close(code, reason);
	}
}

class NativeActorContextAdapter {
	#ctx: NativeActorContext;
	#abortSignal?: AbortSignal;
	#queue?: NativeQueueAdapter;
	#schedule?: NativeScheduleAdapter;

	constructor(ctx: NativeActorContext) {
		this.#ctx = ctx;
	}

	get kv() {
		return this.#ctx.kv();
	}

	get sql() {
		return this.#ctx.sql();
	}

	get state(): unknown {
		return decodeValue(this.#ctx.state());
	}

	set state(value: unknown) {
		this.#ctx.setState(encodeValue(value));
	}

	get vars(): unknown {
		return decodeValue(this.#ctx.vars());
	}

	set vars(value: unknown) {
		this.#ctx.setVars(encodeValue(value));
	}

	get queue(): NativeQueueAdapter {
		if (!this.#queue) {
			this.#queue = new NativeQueueAdapter(this.#ctx.queue());
		}
		return this.#queue;
	}

	get schedule(): NativeScheduleAdapter {
		if (!this.#schedule) {
			this.#schedule = new NativeScheduleAdapter(this.#ctx.schedule());
		}
		return this.#schedule;
	}

	get actorId(): string {
		return this.#ctx.actorId();
	}

	get name(): string {
		return this.#ctx.name();
	}

	get key(): Array<string | number> {
		return toActorKey(this.#ctx.key());
	}

	get region(): string {
		return this.#ctx.region();
	}

	get conns(): Map<string, NativeConnAdapter> {
		return new Map(
			this.#ctx
				.conns()
				.map((conn) => [conn.id(), new NativeConnAdapter(conn)]),
		);
	}

	get log() {
		return logger();
	}

	get abortSignal(): AbortSignal {
		if (!this.#abortSignal) {
			const nativeSignal = this.#ctx.abortSignal();
			const controller = new AbortController();
			if (nativeSignal.aborted()) {
				controller.abort();
			} else {
				nativeSignal.onCancelled(() => controller.abort());
			}
			this.#abortSignal = controller.signal;
		}
		return this.#abortSignal;
	}

	get aborted(): boolean {
		return this.#ctx.aborted();
	}

	broadcast(name: string, ...args: unknown[]): void {
		this.#ctx.broadcast(name, encodeValue(args));
	}

	async saveState(opts?: { immediate?: boolean }): Promise<void> {
		await this.#ctx.saveState(opts?.immediate ?? false);
	}

	waitUntil(promise: Promise<unknown>): void {
		void this.#ctx.waitUntil(Promise.resolve(promise));
	}

	setPreventSleep(preventSleep: boolean): void {
		this.#ctx.setPreventSleep(preventSleep);
	}

	preventSleep(): boolean {
		return this.#ctx.preventSleep();
	}

	sleep(): void {
		this.#ctx.sleep();
	}

	destroy(): void {
		this.#ctx.destroy();
	}

	client(): never {
		throw new Error("c.client() is not wired through the NAPI registry yet");
	}
}

function withConnContext(ctx: NativeActorContext, conn: NativeConnHandle) {
	return Object.assign(new NativeActorContextAdapter(ctx), {
		conn: new NativeConnAdapter(conn),
	});
}

function buildActorConfig(definition: AnyActorDefinition): JsActorConfig {
	const config = definition.config as unknown as Record<string, unknown>;
	const canHibernate = config.canHibernateWebSocket;

	return {
		name: config.name as string | undefined,
		icon: config.icon as string | undefined,
		canHibernateWebsocket:
			typeof canHibernate === "boolean" ? canHibernate : undefined,
		stateSaveIntervalMs: config.stateSaveInterval as number | undefined,
		createVarsTimeoutMs: config.createVarsTimeout as number | undefined,
		createConnStateTimeoutMs: config.createConnStateTimeout as number | undefined,
		onBeforeConnectTimeoutMs:
			config.onBeforeConnectTimeout as number | undefined,
		onConnectTimeoutMs: config.onConnectTimeout as number | undefined,
		onSleepTimeoutMs: config.onSleepTimeout as number | undefined,
		onDestroyTimeoutMs: config.onDestroyTimeout as number | undefined,
		actionTimeoutMs: config.actionTimeout as number | undefined,
		runStopTimeoutMs: config.runStopTimeout as number | undefined,
		sleepTimeoutMs: config.sleepTimeout as number | undefined,
		noSleep: config.noSleep as boolean | undefined,
		sleepGracePeriodMs: config.sleepGracePeriod as number | undefined,
		connectionLivenessTimeoutMs:
			config.connectionLivenessTimeout as number | undefined,
		connectionLivenessIntervalMs:
			config.connectionLivenessInterval as number | undefined,
		maxQueueSize: config.maxQueueSize as number | undefined,
		maxQueueMessageSize: config.maxQueueMessageSize as number | undefined,
		preloadMaxWorkflowBytes:
			config.preloadMaxWorkflowBytes as number | undefined,
		preloadMaxConnectionsBytes:
			config.preloadMaxConnectionsBytes as number | undefined,
	};
}

function buildNativeFactory(
	bindings: NativeBindings,
	definition: AnyActorDefinition,
): NativeActorFactory {
	const config = definition.config as Record<string, any>;
	const callbacks = {
		onInit: async ({
			ctx,
			input,
			isNew,
		}: {
			ctx: NativeActorContext;
			input?: Buffer;
			isNew: boolean;
		}): Promise<JsFactoryInitResult> => {
			const actorCtx = new NativeActorContextAdapter(ctx);
			const decodedInput = decodeValue(input);
			const result: JsFactoryInitResult = {};

			if (isNew) {
				if ("state" in config) {
					result.state = encodeValue(config.state);
					actorCtx.state = config.state;
				} else if (typeof config.createState === "function") {
					const state = await config.createState(actorCtx, decodedInput);
					result.state = encodeValue(state);
					actorCtx.state = state;
				}
			}

			if ("vars" in config) {
				result.vars = encodeValue(config.vars);
				actorCtx.vars = config.vars;
			} else if (typeof config.createVars === "function") {
				const vars = await config.createVars(actorCtx, undefined);
				result.vars = encodeValue(vars);
				actorCtx.vars = vars;
			}

			if (isNew && typeof config.onCreate === "function") {
				await config.onCreate(actorCtx, decodedInput);
				if (actorCtx.state !== undefined) {
					result.state = encodeValue(actorCtx.state);
				}
				if (actorCtx.vars !== undefined) {
					result.vars = encodeValue(actorCtx.vars);
				}
			}

			return result;
		},
		onWake:
			typeof config.onWake === "function"
				? async ({ ctx }: { ctx: NativeActorContext }) =>
						await config.onWake(new NativeActorContextAdapter(ctx))
				: undefined,
		onSleep:
			typeof config.onSleep === "function"
				? async ({ ctx }: { ctx: NativeActorContext }) =>
						await config.onSleep(new NativeActorContextAdapter(ctx))
				: undefined,
		onDestroy:
			typeof config.onDestroy === "function"
				? async ({ ctx }: { ctx: NativeActorContext }) =>
						await config.onDestroy(new NativeActorContextAdapter(ctx))
				: undefined,
		onStateChange:
			typeof config.onStateChange === "function"
				? async ({
						ctx,
						newState,
					}: {
						ctx: NativeActorContext;
						newState: Buffer;
					}) =>
						await config.onStateChange(
							new NativeActorContextAdapter(ctx),
							decodeValue(newState),
						)
				: undefined,
		onBeforeConnect:
			typeof config.onBeforeConnect === "function"
				? async ({
						ctx,
						params,
					}: {
						ctx: NativeActorContext;
						params: Buffer;
					}) =>
						await config.onBeforeConnect(
							new NativeActorContextAdapter(ctx),
							decodeValue(params),
						)
				: undefined,
		onConnect:
			typeof config.onConnect === "function"
				? async ({
						ctx,
						conn,
					}: {
						ctx: NativeActorContext;
						conn: NativeConnHandle;
					}) =>
						await config.onConnect(
							withConnContext(ctx, conn),
							new NativeConnAdapter(conn),
						)
				: undefined,
		onDisconnect:
			typeof config.onDisconnect === "function"
				? async ({
						ctx,
						conn,
					}: {
						ctx: NativeActorContext;
						conn: NativeConnHandle;
					}) =>
						await config.onDisconnect(
							withConnContext(ctx, conn),
							new NativeConnAdapter(conn),
						)
				: undefined,
		onBeforeActionResponse:
			typeof config.onBeforeActionResponse === "function"
				? async ({
						ctx,
						name,
						args,
						output,
					}: {
						ctx: NativeActorContext;
						name: string;
						args: Buffer;
						output: Buffer;
					}) =>
						encodeValue(
							await config.onBeforeActionResponse(
								new NativeActorContextAdapter(ctx),
								name,
								decodeArgs(args),
								decodeValue(output),
							),
						)
				: undefined,
		onRequest:
			typeof config.onRequest === "function"
				? async ({
						ctx,
						request,
					}: {
						ctx: NativeActorContext;
						request: {
							method: string;
							uri: string;
							headers?: Record<string, string>;
							body?: Buffer;
						};
					}) => {
						const jsRequest = buildRequest(request);
						const requestCtx = Object.assign(
							new NativeActorContextAdapter(ctx),
							{ request: jsRequest },
						);
						const response =
							(await config.onRequest(requestCtx, jsRequest)) ??
							new Response(null, { status: 204 });
						return await toJsHttpResponse(
							response,
						);
					}
				: undefined,
		onWebSocket:
			typeof config.onWebSocket === "function"
				? async ({
						ctx,
						ws,
					}: {
						ctx: NativeActorContext;
						ws: NativeWebSocket;
					}) =>
						await config.onWebSocket(
							new NativeActorContextAdapter(ctx),
							new NativeWebSocketAdapter(ws),
						)
				: undefined,
		run: (() => {
			const run = getRunFunction(config.run);
			if (!run) {
				return undefined;
			}

			return async ({ ctx }: { ctx: NativeActorContext }) =>
				await run(new NativeActorContextAdapter(ctx));
			})(),
			actions: Object.fromEntries(
				(
					Object.entries(config.actions ?? {}) as Array<
						[string, (...args: Array<any>) => any]
					>
				).map(([name, handler]) => [
					name,
					async ({
						ctx,
						conn,
					args,
				}: {
					ctx: NativeActorContext;
					conn: NativeConnHandle;
					args: Buffer;
				}) =>
					encodeValue(
						await handler(
							withConnContext(ctx, conn),
							...decodeArgs(args),
						),
					),
				]),
			),
		};

	return new bindings.NapiActorFactory(callbacks, buildActorConfig(definition));
}

async function buildServeConfig(config: RegistryConfig): Promise<JsServeConfig> {
	if (!config.endpoint) {
		throw new Error("registry endpoint is required for native envoy startup");
	}

	const serveConfig: JsServeConfig = {
		version: config.envoy.version,
		endpoint: config.endpoint,
		token: config.token,
		namespace: config.namespace,
		poolName: config.envoy.poolName,
	};

	if (config.startEngine) {
		const { getEnginePath } = await loadEngineCli();
		serveConfig.engineBinaryPath = getEnginePath();
	}

	return serveConfig;
}

export async function buildNativeRegistry(config: RegistryConfig): Promise<{
	registry: NativeCoreRegistry;
	serveConfig: JsServeConfig;
}> {
	const bindings = await loadNativeBindings();
	const registry = new bindings.CoreRegistry();

	for (const [name, definition] of Object.entries(config.use)) {
		registry.register(name, buildNativeFactory(bindings, definition));
	}

	return {
		registry,
		serveConfig: await buildServeConfig(config),
	};
}
