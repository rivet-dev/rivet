import * as cbor from "cbor-x";
import type { Encoding } from "@/common/encoding";
import { HEADER_ENCODING } from "@/common/actor-router-consts";
import {
	CURRENT_VERSION as CLIENT_PROTOCOL_CURRENT_VERSION,
	HTTP_ACTION_REQUEST_VERSIONED,
	HTTP_ACTION_RESPONSE_VERSIONED,
} from "@/common/client-protocol-versioned";
import {
	HttpActionRequestSchema,
	HttpActionResponseSchema,
} from "@/common/client-protocol-zod";
import { getRunFunction } from "@/actor/config";
import type { AnyActorDefinition } from "@/actor/definition";
import { wrapJsNativeDatabase } from "@/common/database/native-database";
import type { RegistryConfig } from "@/registry/config";
import {
	contentTypeForEncoding,
	deserializeWithEncoding,
	serializeWithEncoding,
} from "@/serde";
import { bufferToArrayBuffer } from "@/utils";
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
const textEncoder = new TextEncoder();
const nativeSqlDatabases = new Map<
	string,
	ReturnType<typeof wrapJsNativeDatabase>
>();

function closeNativeSqlDatabase(actorId: string): Promise<void> | undefined {
	const database = nativeSqlDatabases.get(actorId);
	if (!database) {
		return;
	}

	nativeSqlDatabases.delete(actorId);
	return database.close();
}

function toBuffer(value: string | Uint8Array | ArrayBuffer): Buffer {
	if (typeof value === "string") {
		return Buffer.from(textEncoder.encode(value));
	}
	if (value instanceof Uint8Array) {
		return Buffer.from(value);
	}
	return Buffer.from(value);
}

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

function createWriteThroughProxy<T>(
	value: T,
	commit: (next: T) => void,
): T {
	if (!value || typeof value !== "object") {
		return value;
	}

	const proxies = new WeakMap<object, object>();
	const wrap = (target: object): object => {
		const cached = proxies.get(target);
		if (cached) {
			return cached;
		}

		const proxy = new Proxy(target, {
			get(innerTarget, property, receiver) {
				const result = Reflect.get(innerTarget, property, receiver);
				return result && typeof result === "object"
					? wrap(result as object)
					: result;
			},
			set(innerTarget, property, nextValue, receiver) {
				const updated = Reflect.set(
					innerTarget,
					property,
					nextValue,
					receiver,
				);
				commit(value);
				return updated;
			},
			deleteProperty(innerTarget, property) {
				const updated = Reflect.deleteProperty(innerTarget, property);
				commit(value);
				return updated;
			},
		});

		proxies.set(target, proxy);
		return proxy;
	};

	return wrap(value as object) as T;
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
		return createWriteThroughProxy(
			decodeValue(this.#conn.state()),
			(nextValue) => this.#conn.setState(encodeValue(nextValue)),
		);
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

class NativeKvAdapter {
	#kv: ReturnType<NativeActorContext["kv"]>;

	constructor(kv: ReturnType<NativeActorContext["kv"]>) {
		this.#kv = kv;
	}

	async get(key: string | Uint8Array): Promise<Uint8Array | null> {
		const value = await this.#kv.get(toBuffer(key));
		return value ? new Uint8Array(value) : null;
	}

	async put(
		key: string | Uint8Array,
		value: string | Uint8Array | ArrayBuffer,
	): Promise<void> {
		await this.#kv.put(toBuffer(key), toBuffer(value));
	}

	async delete(key: string | Uint8Array): Promise<void> {
		await this.#kv.delete(toBuffer(key));
	}

	async deleteRange(
		start: string | Uint8Array,
		end: string | Uint8Array,
	): Promise<void> {
		await this.#kv.deleteRange(toBuffer(start), toBuffer(end));
	}

	async listPrefix(
		prefix: string | Uint8Array,
		options?: { reverse?: boolean; limit?: number },
	): Promise<Array<[Uint8Array, Uint8Array]>> {
		const entries = await this.#kv.listPrefix(toBuffer(prefix), options);
		return entries.map((entry) => [
			new Uint8Array(entry.key),
			new Uint8Array(entry.value),
		]);
	}

	async listRange(
		start: string | Uint8Array,
		end: string | Uint8Array,
		options?: { reverse?: boolean; limit?: number },
	): Promise<Array<[Uint8Array, Uint8Array]>> {
		const entries = await this.#kv.listRange(
			toBuffer(start),
			toBuffer(end),
			options,
		);
		return entries.map((entry) => [
			new Uint8Array(entry.key),
			new Uint8Array(entry.value),
		]);
	}

	async list(
		prefix: string | Uint8Array,
		options?: { reverse?: boolean; limit?: number },
	): Promise<Array<[Uint8Array, Uint8Array]>> {
		return this.listPrefix(prefix, options);
	}

	async batchGet(keys: Uint8Array[]): Promise<Array<Uint8Array | null>> {
		const values = await this.#kv.batchGet(keys.map((key) => Buffer.from(key)));
		return values.map((value) => (value ? new Uint8Array(value) : null));
	}

	async batchPut(entries: [Uint8Array, Uint8Array][]): Promise<void> {
		await this.#kv.batchPut(
			entries.map(([key, value]) => ({
				key: Buffer.from(key),
				value: Buffer.from(value),
			})),
		);
	}

	async batchDelete(keys: Uint8Array[]): Promise<void> {
		await this.#kv.batchDelete(keys.map((key) => Buffer.from(key)));
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
	#kv?: NativeKvAdapter;
	#queue?: NativeQueueAdapter;
	#schedule?: NativeScheduleAdapter;
	#sql?: ReturnType<typeof wrapJsNativeDatabase>;

	constructor(ctx: NativeActorContext) {
		this.#ctx = ctx;
	}

	get kv() {
		if (!this.#kv) {
			this.#kv = new NativeKvAdapter(this.#ctx.kv());
		}
		return this.#kv;
	}

	get sql() {
		if (!this.#sql) {
			const actorId = this.#ctx.actorId();
			const cachedDatabase = nativeSqlDatabases.get(actorId);
			if (cachedDatabase) {
				this.#sql = cachedDatabase;
			} else {
				const database = wrapJsNativeDatabase(this.#ctx.sql());
				nativeSqlDatabases.set(actorId, database);
				this.#sql = database;
			}
		}
		return this.#sql;
	}

	get state(): unknown {
		return createWriteThroughProxy(
			decodeValue(this.#ctx.state()),
			(nextValue) => this.#ctx.setState(encodeValue(nextValue)),
		);
	}

	set state(value: unknown) {
		this.#ctx.setState(encodeValue(value));
	}

	get vars(): unknown {
		return createWriteThroughProxy(
			decodeValue(this.#ctx.vars()),
			(nextValue) => this.#ctx.setVars(encodeValue(nextValue)),
		);
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
		const closeDatabase = closeNativeSqlDatabase(this.actorId);
		if (closeDatabase) {
			this.waitUntil(closeDatabase);
		}
		this.#ctx.sleep();
	}

	destroy(): void {
		const closeDatabase = closeNativeSqlDatabase(this.actorId);
		if (closeDatabase) {
			this.waitUntil(closeDatabase);
		}
		this.#ctx.destroy();
	}

	client(): never {
		throw new Error("c.client() is not wired through the NAPI registry yet");
	}

	async dispose(): Promise<void> {
		this.#sql = undefined;
	}
}

function withConnContext(ctx: NativeActorContext, conn: NativeConnHandle) {
	return Object.assign(new NativeActorContextAdapter(ctx), {
		conn: new NativeConnAdapter(conn),
	});
}

async function maybeHandleNativeActionRequest(
	ctx: NativeActorContext,
	request: Request,
	actions: Record<string, (...args: Array<any>) => any>,
): Promise<Response | undefined> {
	if (request.method !== "POST") {
		return undefined;
	}

	const actionMatch = /^\/action\/([^/]+)$/.exec(new URL(request.url).pathname);
	if (!actionMatch) {
		return undefined;
	}

	const handler = actions[decodeURIComponent(actionMatch[1] ?? "")];
	if (typeof handler !== "function") {
		return new Response(null, { status: 404 });
	}

	const encodingHeader = request.headers.get(HEADER_ENCODING);
	const encoding: Encoding =
		encodingHeader === "cbor" || encodingHeader === "bare"
			? encodingHeader
			: "json";
	const requestBody = new Uint8Array(await request.arrayBuffer());
	const args = deserializeWithEncoding(
		encoding,
		encoding === "json"
			? new TextDecoder().decode(requestBody)
			: requestBody,
		HTTP_ACTION_REQUEST_VERSIONED,
		HttpActionRequestSchema,
		(json) => (Array.isArray(json.args) ? json.args : []),
		(bare) =>
			bare.args ? (cbor.decode(new Uint8Array(bare.args)) as unknown[]) : [],
	);
	const actorCtx = new NativeActorContextAdapter(ctx);
	let output: unknown;
	try {
		output = await handler(actorCtx, ...args);
	} finally {
		await actorCtx.dispose();
	}
	const responseBody = serializeWithEncoding<
		{ output: ArrayBuffer },
		{ output: unknown },
		unknown
	>(
		encoding,
		output,
		HTTP_ACTION_RESPONSE_VERSIONED,
		CLIENT_PROTOCOL_CURRENT_VERSION,
		HttpActionResponseSchema,
		(value) => ({ output: value }),
		(value) => ({
			output: bufferToArrayBuffer(cbor.encode(value)),
		}),
	);

	return new Response(responseBody, {
		status: 200,
		headers: {
			"Content-Type": contentTypeForEncoding(encoding),
		},
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
	const actionHandlers = Object.fromEntries(
		(
			Object.entries(config.actions ?? {}) as Array<
				[string, (...args: Array<any>) => any]
			>
		).map(([name, handler]) => [name, handler]),
	);
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
			try {
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
			} finally {
				await actorCtx.dispose();
			}
		},
		onWake:
			typeof config.onWake === "function"
				? async ({ ctx }: { ctx: NativeActorContext }) => {
						const actorCtx = new NativeActorContextAdapter(ctx);
						try {
							await config.onWake(actorCtx);
						} finally {
							await actorCtx.dispose();
						}
					}
				: undefined,
		onSleep:
			typeof config.onSleep === "function"
				? async ({ ctx }: { ctx: NativeActorContext }) => {
						const actorCtx = new NativeActorContextAdapter(ctx);
						try {
							await config.onSleep(actorCtx);
						} finally {
							await actorCtx.dispose();
						}
					}
				: undefined,
		onDestroy:
			typeof config.onDestroy === "function"
				? async ({ ctx }: { ctx: NativeActorContext }) => {
						const actorCtx = new NativeActorContextAdapter(ctx);
						try {
							await config.onDestroy(actorCtx);
						} finally {
							await actorCtx.dispose();
						}
					}
				: undefined,
		onStateChange:
			typeof config.onStateChange === "function"
				? async ({
						ctx,
						newState,
					}: {
						ctx: NativeActorContext;
						newState: Buffer;
					}) => {
						const actorCtx = new NativeActorContextAdapter(ctx);
						try {
							await config.onStateChange(actorCtx, decodeValue(newState));
						} finally {
							await actorCtx.dispose();
						}
					}
				: undefined,
		onBeforeConnect:
			typeof config.onBeforeConnect === "function"
				? async ({
						ctx,
						params,
					}: {
						ctx: NativeActorContext;
						params: Buffer;
					}) => {
						const actorCtx = new NativeActorContextAdapter(ctx);
						try {
							await config.onBeforeConnect(actorCtx, decodeValue(params));
						} finally {
							await actorCtx.dispose();
						}
					}
				: undefined,
		onConnect:
			typeof config.onConnect === "function"
				? async ({
						ctx,
						conn,
					}: {
						ctx: NativeActorContext;
						conn: NativeConnHandle;
					}) => {
						const actorCtx = withConnContext(ctx, conn);
						try {
							await config.onConnect(actorCtx, new NativeConnAdapter(conn));
						} finally {
							await actorCtx.dispose();
						}
					}
				: undefined,
		onDisconnect:
			typeof config.onDisconnect === "function"
				? async ({
						ctx,
						conn,
					}: {
						ctx: NativeActorContext;
						conn: NativeConnHandle;
					}) => {
						const actorCtx = withConnContext(ctx, conn);
						try {
							await config.onDisconnect(actorCtx, new NativeConnAdapter(conn));
						} finally {
							await actorCtx.dispose();
						}
					}
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
					}) => {
						const actorCtx = new NativeActorContextAdapter(ctx);
						try {
							return encodeValue(
								await config.onBeforeActionResponse(
									actorCtx,
									name,
									decodeArgs(args),
									decodeValue(output),
								),
							);
						} finally {
							await actorCtx.dispose();
						}
					}
				: undefined,
		onRequest: async ({
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
			try {
				const jsRequest = buildRequest(request);
				const actionResponse = await maybeHandleNativeActionRequest(
					ctx,
					jsRequest,
					actionHandlers,
				);
				if (actionResponse) {
					return await toJsHttpResponse(actionResponse);
				}

				if (typeof config.onRequest !== "function") {
					return await toJsHttpResponse(new Response(null, { status: 404 }));
				}

				const requestCtx = Object.assign(new NativeActorContextAdapter(ctx), {
					request: jsRequest,
				});
				try {
					const response =
						(await config.onRequest(requestCtx, jsRequest)) ??
						new Response(null, { status: 204 });
					return await toJsHttpResponse(response);
				} finally {
					await requestCtx.dispose();
				}
			} catch (error) {
				logger().error({
					msg: "native onRequest failed",
					error,
				});
				throw error;
			}
		},
		onWebSocket:
			typeof config.onWebSocket === "function"
				? async ({
						ctx,
						ws,
					}: {
						ctx: NativeActorContext;
						ws: NativeWebSocket;
					}) => {
						const actorCtx = new NativeActorContextAdapter(ctx);
						try {
							await config.onWebSocket(
								actorCtx,
								new NativeWebSocketAdapter(ws),
							);
						} finally {
							await actorCtx.dispose();
						}
					}
				: undefined,
		run: (() => {
			const run = getRunFunction(config.run);
			if (!run) {
				return undefined;
			}

			return async ({ ctx }: { ctx: NativeActorContext }) => {
				const actorCtx = new NativeActorContextAdapter(ctx);
				try {
					await run(actorCtx);
				} finally {
					await actorCtx.dispose();
				}
			};
			})(),
			actions: Object.fromEntries(
				Object.entries(actionHandlers).map(([name, handler]) => [
					name,
					async ({
						ctx,
						conn,
						args,
					}: {
						ctx: NativeActorContext;
						conn: NativeConnHandle;
						args: Buffer;
					}) => {
						const actorCtx = withConnContext(ctx, conn);
						try {
							return encodeValue(
								await handler(actorCtx, ...decodeArgs(args)),
							);
						} finally {
							await actorCtx.dispose();
						}
					},
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
