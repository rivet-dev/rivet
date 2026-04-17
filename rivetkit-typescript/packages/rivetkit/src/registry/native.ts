import * as cbor from "cbor-x";
import type { Encoding } from "@/common/encoding";
import { HEADER_ENCODING } from "@/common/actor-router-consts";
import {
	CURRENT_VERSION as CLIENT_PROTOCOL_CURRENT_VERSION,
	HTTP_ACTION_REQUEST_VERSIONED,
	HTTP_ACTION_RESPONSE_VERSIONED,
	HTTP_RESPONSE_ERROR_VERSIONED,
} from "@/common/client-protocol-versioned";
import {
	HttpActionRequestSchema,
	HttpActionResponseSchema,
	type HttpResponseError as HttpResponseErrorJson,
	HttpResponseErrorSchema,
} from "@/common/client-protocol-zod";
import type * as protocol from "@/common/client-protocol";
import { getRunFunction } from "@/actor/config";
import type { AnyActorDefinition } from "@/actor/definition";
import {
	decodeBridgeRivetError,
	encodeBridgeRivetError,
	toRivetError,
} from "@/actor/errors";
import { wrapJsNativeDatabase } from "@/common/database/native-database";
import { deconstructError } from "@/common/utils";
import {
	type AnyClient,
	createClientWithDriver,
} from "@/client/client";
import { convertRegistryConfigToClientConfig } from "@/client/config";
import { RemoteEngineControlClient } from "@/engine-client/mod";
import type { RegistryConfig } from "@/registry/config";
import {
	contentTypeForEncoding,
	deserializeWithEncoding,
	serializeWithEncoding,
} from "@/serde";
import { bufferToArrayBuffer } from "@/utils";
import { logger } from "./log";
import {
	type NativeValidationConfig,
	validateActionArgs,
	validateConnParams,
	validateEventArgs,
	validateQueueBody,
} from "./native-validation";

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

function unwrapTsfnPayload<T>(error: unknown, payload: T): T {
	if (error !== null && error !== undefined) {
		throw error;
	}

	return payload;
}

function normalizeNativeBridgeError(error: unknown): unknown {
	if (typeof error === "string") {
		return decodeBridgeRivetError(error) ?? error;
	}

	if (error instanceof Error) {
		const bridged = decodeBridgeRivetError(error.message);
		if (bridged) {
			return bridged;
		}
	}

	return error;
}

function encodeNativeCallbackError(error: unknown): Error {
	const normalized = toRivetError(error, {
		group: "actor",
		code: "internal_error",
		message:
			error instanceof Error ? error.message : `Internal error: ${String(error)}`,
	});
	const bridgeError = new Error(encodeBridgeRivetError(normalized), {
		cause: error instanceof Error ? error : undefined,
	});
	return Object.assign(bridgeError, {
		group: normalized.group,
		code: normalized.code,
		metadata: normalized.metadata,
	});
}

async function callNative<T>(invoke: () => Promise<T>): Promise<T> {
	try {
		return await invoke();
	} catch (error) {
		throw normalizeNativeBridgeError(error);
	}
}

function callNativeSync<T>(invoke: () => T): T {
	try {
		return invoke();
	} catch (error) {
		throw normalizeNativeBridgeError(error);
	}
}

function wrapNativeCallback<Args extends Array<unknown>, Result>(
	callback: (...args: Args) => Result | Promise<Result>,
): (...args: Args) => Promise<Result> {
	return async (...args: Args) => {
		try {
			return await callback(...args);
		} catch (error) {
			throw encodeNativeCallbackError(error);
		}
	};
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
	#schemas: NativeValidationConfig;

	constructor(conn: NativeConnHandle, schemas: NativeValidationConfig = {}) {
		this.#conn = conn;
		this.#schemas = schemas;
	}

	get id(): string {
		return this.#conn.id();
	}

	get params(): unknown {
		return validateConnParams(
			this.#schemas.connParamsSchema,
			decodeValue(this.#conn.params()),
		);
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
		return callNativeSync(() => this.#conn.isHibernatable());
	}

	send(name: string, ...args: unknown[]): void {
		const validatedArgs = validateEventArgs(this.#schemas.events, name, args);
		callNativeSync(() => this.#conn.send(name, encodeValue(validatedArgs)));
	}

	async disconnect(reason?: string): Promise<void> {
		await callNative(() => this.#conn.disconnect(reason));
	}
}

class NativeScheduleAdapter {
	#schedule: NativeSchedule;

	constructor(schedule: NativeSchedule) {
		this.#schedule = schedule;
	}

	async after(duration: number, action: string, ...args: unknown[]): Promise<void> {
		callNativeSync(() =>
			this.#schedule.after(duration, action, encodeValue(args)),
		);
	}

	async at(timestamp: number, action: string, ...args: unknown[]): Promise<void> {
		callNativeSync(() =>
			this.#schedule.at(timestamp, action, encodeValue(args)),
		);
	}
}

class NativeKvAdapter {
	#kv: ReturnType<NativeActorContext["kv"]>;

	constructor(kv: ReturnType<NativeActorContext["kv"]>) {
		this.#kv = kv;
	}

	async get(key: string | Uint8Array): Promise<Uint8Array | null> {
		const value = await callNative(() => this.#kv.get(toBuffer(key)));
		return value ? new Uint8Array(value) : null;
	}

	async put(
		key: string | Uint8Array,
		value: string | Uint8Array | ArrayBuffer,
	): Promise<void> {
		await callNative(() => this.#kv.put(toBuffer(key), toBuffer(value)));
	}

	async delete(key: string | Uint8Array): Promise<void> {
		await callNative(() => this.#kv.delete(toBuffer(key)));
	}

	async deleteRange(
		start: string | Uint8Array,
		end: string | Uint8Array,
	): Promise<void> {
		await callNative(() =>
			this.#kv.deleteRange(toBuffer(start), toBuffer(end)),
		);
	}

	async listPrefix(
		prefix: string | Uint8Array,
		options?: { reverse?: boolean; limit?: number },
	): Promise<Array<[Uint8Array, Uint8Array]>> {
		const entries = await callNative(() =>
			this.#kv.listPrefix(toBuffer(prefix), options),
		);
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
		const entries = await callNative(() =>
			this.#kv.listRange(toBuffer(start), toBuffer(end), options),
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
		const values = await callNative(() =>
			this.#kv.batchGet(keys.map((key) => Buffer.from(key))),
		);
		return values.map((value) => (value ? new Uint8Array(value) : null));
	}

	async batchPut(entries: [Uint8Array, Uint8Array][]): Promise<void> {
		await callNative(() =>
			this.#kv.batchPut(
				entries.map(([key, value]) => ({
					key: Buffer.from(key),
					value: Buffer.from(value),
				})),
			),
		);
	}

	async batchDelete(keys: Uint8Array[]): Promise<void> {
		await callNative(() =>
			this.#kv.batchDelete(keys.map((key) => Buffer.from(key))),
		);
	}
}

function wrapQueueMessage(
	message: NativeQueueMessage,
	schemas: NativeValidationConfig["queues"],
) {
	const name = callNativeSync(() => message.name());
	return {
		id: Number(callNativeSync(() => message.id())),
		name,
		body: validateQueueBody(
			schemas,
			name,
			decodeValue(callNativeSync(() => message.body())),
		),
		createdAt: callNativeSync(() => message.createdAt()),
		complete: callNativeSync(() => message.isCompletable())
			? async (response?: unknown) =>
					await callNative(() =>
						message.complete(
							response === undefined
								? undefined
								: encodeValue(response),
						),
					)
			: undefined,
	};
}

class NativeQueueAdapter {
	#queue: NativeQueue;
	#schemas: NativeValidationConfig["queues"];

	constructor(
		queue: NativeQueue,
		schemas: NativeValidationConfig["queues"] = undefined,
	) {
		this.#queue = queue;
		this.#schemas = schemas;
	}

	async send(name: string, body: unknown) {
		const validatedBody = validateQueueBody(this.#schemas, name, body);
		return wrapQueueMessage(
			await callNative(() =>
				this.#queue.send(name, encodeValue(validatedBody)),
			),
			this.#schemas,
		);
	}

	async next(options?: {
		names?: readonly string[];
		timeout?: number;
		completable?: boolean;
	}) {
		const message = await callNative(() =>
			this.#queue.next({
				names: options?.names ? [...options.names] : undefined,
				timeoutMs: options?.timeout,
				completable: options?.completable,
			}),
		);
		return message ? wrapQueueMessage(message, this.#schemas) : undefined;
	}

	async nextBatch(options?: {
		names?: readonly string[];
		count?: number;
		timeout?: number;
		completable?: boolean;
	}) {
		const messages = await callNative(() =>
			this.#queue.nextBatch({
				names: options?.names ? [...options.names] : undefined,
				count: options?.count,
				timeoutMs: options?.timeout,
				completable: options?.completable,
			}),
		);
		return messages.map((message) => wrapQueueMessage(message, this.#schemas));
	}

	async tryNext(options?: {
		names?: readonly string[];
		completable?: boolean;
	}) {
		const message = callNativeSync(() =>
			this.#queue.tryNext({
				names: options?.names ? [...options.names] : undefined,
				completable: options?.completable,
			}),
		);
		return message ? wrapQueueMessage(message, this.#schemas) : undefined;
	}

	async tryNextBatch(options?: {
		names?: readonly string[];
		count?: number;
		completable?: boolean;
	}) {
		const messages = callNativeSync(() =>
			this.#queue.tryNextBatch({
				names: options?.names ? [...options.names] : undefined,
				count: options?.count,
				completable: options?.completable,
			}),
		);
		return messages.map((message) => wrapQueueMessage(message, this.#schemas));
	}
}

class NativeWebSocketAdapter {
	#ws: NativeWebSocket;

	constructor(ws: NativeWebSocket) {
		this.#ws = ws;
	}

	send(data: string | ArrayBuffer | ArrayBufferView): void {
		if (typeof data === "string") {
			callNativeSync(() => this.#ws.send(Buffer.from(data), false));
			return;
		}

		const buffer = ArrayBuffer.isView(data)
			? Buffer.from(data.buffer, data.byteOffset, data.byteLength)
			: Buffer.from(data);
		callNativeSync(() => this.#ws.send(buffer, true));
	}

	close(code?: number, reason?: string): void {
		callNativeSync(() => this.#ws.close(code, reason));
	}
}

class NativeActorContextAdapter {
	#ctx: NativeActorContext;
	#schemas: NativeValidationConfig;
	#abortSignal?: AbortSignal;
	#client?: AnyClient;
	#clientFactory?: () => AnyClient;
	#kv?: NativeKvAdapter;
	#queue?: NativeQueueAdapter;
	#schedule?: NativeScheduleAdapter;
	#sql?: ReturnType<typeof wrapJsNativeDatabase>;

	constructor(
		ctx: NativeActorContext,
		clientFactory?: () => AnyClient,
		schemas: NativeValidationConfig = {},
	) {
		this.#ctx = ctx;
		this.#clientFactory = clientFactory;
		this.#schemas = schemas;
	}

	get kv() {
		if (!this.#kv) {
			this.#kv = new NativeKvAdapter(this.#ctx.kv());
		}
		return this.#kv;
	}

	get sql() {
		if (!this.#sql) {
			const actorId = callNativeSync(() => this.#ctx.actorId());
			const cachedDatabase = nativeSqlDatabases.get(actorId);
			if (cachedDatabase) {
				this.#sql = cachedDatabase;
			} else {
				const database = wrapJsNativeDatabase(
					callNativeSync(() => this.#ctx.sql()),
				);
				nativeSqlDatabases.set(actorId, database);
				this.#sql = database;
			}
		}
		return this.#sql;
	}

	get state(): unknown {
		return createWriteThroughProxy(
			decodeValue(callNativeSync(() => this.#ctx.state())),
			(nextValue) =>
				callNativeSync(() => this.#ctx.setState(encodeValue(nextValue))),
		);
	}

	set state(value: unknown) {
		callNativeSync(() => this.#ctx.setState(encodeValue(value)));
	}

	get vars(): unknown {
		return createWriteThroughProxy(
			decodeValue(callNativeSync(() => this.#ctx.vars())),
			(nextValue) =>
				callNativeSync(() => this.#ctx.setVars(encodeValue(nextValue))),
		);
	}

	set vars(value: unknown) {
		callNativeSync(() => this.#ctx.setVars(encodeValue(value)));
	}

	get queue(): NativeQueueAdapter {
		if (!this.#queue) {
			this.#queue = new NativeQueueAdapter(
				callNativeSync(() => this.#ctx.queue()),
				this.#schemas.queues,
			);
		}
		return this.#queue;
	}

	get schedule(): NativeScheduleAdapter {
		if (!this.#schedule) {
			this.#schedule = new NativeScheduleAdapter(
				callNativeSync(() => this.#ctx.schedule()),
			);
		}
		return this.#schedule;
	}

	get actorId(): string {
		return callNativeSync(() => this.#ctx.actorId());
	}

	get name(): string {
		return callNativeSync(() => this.#ctx.name());
	}

	get key(): Array<string | number> {
		return toActorKey(callNativeSync(() => this.#ctx.key()));
	}

	get region(): string {
		return callNativeSync(() => this.#ctx.region());
	}

	get conns(): Map<string, NativeConnAdapter> {
		return new Map(
			callNativeSync(() => this.#ctx.conns())
				.map((conn) => [conn.id(), new NativeConnAdapter(conn, this.#schemas)]),
		);
	}

	get log() {
		return logger();
	}

	get abortSignal(): AbortSignal {
		if (!this.#abortSignal) {
			const nativeSignal = callNativeSync(() => this.#ctx.abortSignal());
			const controller = new AbortController();
			if (callNativeSync(() => nativeSignal.aborted())) {
				controller.abort();
			} else {
				callNativeSync(() =>
					nativeSignal.onCancelled(() => controller.abort()),
				);
			}
			this.#abortSignal = controller.signal;
		}
		return this.#abortSignal;
	}

	get aborted(): boolean {
		return callNativeSync(() => this.#ctx.aborted());
	}

	broadcast(name: string, ...args: unknown[]): void {
		const validatedArgs = validateEventArgs(this.#schemas.events, name, args);
		callNativeSync(() =>
			this.#ctx.broadcast(name, encodeValue(validatedArgs)),
		);
	}

	async saveState(opts?: { immediate?: boolean }): Promise<void> {
		await callNative(() => this.#ctx.saveState(opts?.immediate ?? false));
	}

	waitUntil(promise: Promise<unknown>): void {
		void callNative(() => this.#ctx.waitUntil(Promise.resolve(promise)));
	}

	setPreventSleep(preventSleep: boolean): void {
		callNativeSync(() => this.#ctx.setPreventSleep(preventSleep));
	}

	preventSleep(): boolean {
		return callNativeSync(() => this.#ctx.preventSleep());
	}

	sleep(): void {
		const closeDatabase = closeNativeSqlDatabase(this.actorId);
		if (closeDatabase) {
			this.waitUntil(closeDatabase);
		}
		callNativeSync(() => this.#ctx.sleep());
	}

	destroy(): void {
		const closeDatabase = closeNativeSqlDatabase(this.actorId);
		if (closeDatabase) {
			this.waitUntil(closeDatabase);
		}
		callNativeSync(() => this.#ctx.destroy());
	}

	client<T = AnyClient>(): T {
		if (!this.#client) {
			if (!this.#clientFactory) {
				throw new Error("native actor client is not configured");
			}
			this.#client = this.#clientFactory();
		}

		return this.#client as T;
	}

	async dispose(): Promise<void> {
		this.#sql = undefined;
	}
}

function withConnContext(
	ctx: NativeActorContext,
	conn: NativeConnHandle,
	clientFactory?: () => AnyClient,
	schemas: NativeValidationConfig = {},
) {
	return Object.assign(new NativeActorContextAdapter(ctx, clientFactory, schemas), {
		conn: new NativeConnAdapter(conn, schemas),
	});
}

function buildNativeActionErrorResponse(
	encoding: Encoding,
	actionName: string,
	error: unknown,
): Response {
	const { statusCode, group, code, message, metadata } = deconstructError(
		error,
		logger(),
		{
			actionName,
			path: `/action/${actionName}`,
			runtime: "native",
		},
		true,
	);
	const body = serializeWithEncoding<
		protocol.HttpResponseError,
		HttpResponseErrorJson,
		{ group: string; code: string; message: string; metadata?: unknown }
	>(
		encoding,
		{ group, code, message, metadata },
		HTTP_RESPONSE_ERROR_VERSIONED,
		CLIENT_PROTOCOL_CURRENT_VERSION,
		HttpResponseErrorSchema,
		(value) => value,
		(value) => ({
			group: value.group,
			code: value.code,
			message: value.message,
			metadata:
				value.metadata === undefined
					? null
					: bufferToArrayBuffer(cbor.encode(value.metadata)),
		}),
	);

	return new Response(body, {
		status: statusCode,
		headers: {
			"Content-Type": contentTypeForEncoding(encoding),
		},
	});
}

async function maybeHandleNativeActionRequest(
	ctx: NativeActorContext,
	request: Request,
	clientFactory: () => AnyClient,
	actions: Record<string, (...args: Array<any>) => any>,
	schemas: NativeValidationConfig,
): Promise<Response | undefined> {
	if (request.method !== "POST") {
		return undefined;
	}

	const actionMatch = /^\/action\/([^/]+)$/.exec(new URL(request.url).pathname);
	if (!actionMatch) {
		return undefined;
	}

	const encodingHeader = request.headers.get(HEADER_ENCODING);
	const encoding: Encoding =
		encodingHeader === "cbor" || encodingHeader === "bare"
			? encodingHeader
			: "json";
	const actionName = decodeURIComponent(actionMatch[1] ?? "");
	const handler = actions[actionName];
	if (typeof handler !== "function") {
		return buildNativeActionErrorResponse(encoding, actionName, {
			__type: "ActorError",
			public: true,
			statusCode: 404,
			group: "actor",
			code: "action_not_found",
			message: `action \`${actionName}\` was not found`,
		});
	}
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
	const actorCtx = new NativeActorContextAdapter(ctx, clientFactory, schemas);
	let output: unknown;
	try {
		output = await handler(
			actorCtx,
			...validateActionArgs(schemas.actionInputSchemas, actionName, args),
		);
	} catch (error) {
		return buildNativeActionErrorResponse(encoding, actionName, error);
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
	const options = (config.options ?? {}) as Record<string, unknown>;
	const canHibernate = options.canHibernateWebSocket;

	return {
		name: options.name as string | undefined,
		icon: options.icon as string | undefined,
		canHibernateWebsocket:
			typeof canHibernate === "boolean" ? canHibernate : undefined,
		stateSaveIntervalMs: options.stateSaveInterval as number | undefined,
		createVarsTimeoutMs: options.createVarsTimeout as number | undefined,
		createConnStateTimeoutMs:
			options.createConnStateTimeout as number | undefined,
		onBeforeConnectTimeoutMs:
			options.onBeforeConnectTimeout as number | undefined,
		onConnectTimeoutMs: options.onConnectTimeout as number | undefined,
		onMigrateTimeoutMs: options.onMigrateTimeout as number | undefined,
		onSleepTimeoutMs: options.onSleepTimeout as number | undefined,
		onDestroyTimeoutMs: options.onDestroyTimeout as number | undefined,
		actionTimeoutMs: options.actionTimeout as number | undefined,
		runStopTimeoutMs: options.runStopTimeout as number | undefined,
		sleepTimeoutMs: options.sleepTimeout as number | undefined,
		noSleep: options.noSleep as boolean | undefined,
		sleepGracePeriodMs: options.sleepGracePeriod as number | undefined,
		connectionLivenessTimeoutMs:
			options.connectionLivenessTimeout as number | undefined,
		connectionLivenessIntervalMs:
			options.connectionLivenessInterval as number | undefined,
		maxQueueSize: options.maxQueueSize as number | undefined,
		maxQueueMessageSize: options.maxQueueMessageSize as number | undefined,
		preloadMaxWorkflowBytes:
			options.preloadMaxWorkflowBytes as number | undefined,
		preloadMaxConnectionsBytes:
			options.preloadMaxConnectionsBytes as number | undefined,
	};
}

function buildNativeFactory(
	bindings: NativeBindings,
	registryConfig: RegistryConfig,
	definition: AnyActorDefinition,
): NativeActorFactory {
	const config = definition.config as Record<string, any>;
	const schemaConfig: NativeValidationConfig = {
		actionInputSchemas: config.actionInputSchemas,
		connParamsSchema: config.connParamsSchema,
		events: config.events,
		queues: config.queues,
	};
	const actionHandlers = Object.fromEntries(
		(
			Object.entries(config.actions ?? {}) as Array<
				[string, (...args: Array<any>) => any]
			>
		).map(([name, handler]) => [name, handler]),
	);
	const createClient = () =>
		createClientWithDriver(
			new RemoteEngineControlClient(
				convertRegistryConfigToClientConfig(registryConfig),
			),
			{ encoding: "bare" },
		);
	const callbacks = {
		onInit: wrapNativeCallback(async (
			error: unknown,
			payload: {
				ctx: NativeActorContext;
				input?: Buffer;
				isNew: boolean;
			},
		): Promise<JsFactoryInitResult> => {
			const { ctx, input, isNew } = unwrapTsfnPayload(error, payload);
			const actorCtx = new NativeActorContextAdapter(
				ctx,
				createClient,
				schemaConfig,
			);
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
		}),
		onWake:
			typeof config.onWake === "function"
				? wrapNativeCallback(async (
						error: unknown,
						payload: { ctx: NativeActorContext },
					) => {
						const { ctx } = unwrapTsfnPayload(error, payload);
						const actorCtx = new NativeActorContextAdapter(
							ctx,
							createClient,
							schemaConfig,
						);
						try {
							await config.onWake(actorCtx);
						} finally {
							await actorCtx.dispose();
						}
					})
				: undefined,
		onMigrate:
			typeof config.onMigrate === "function"
				? wrapNativeCallback(async (
						error: unknown,
						payload: {
							ctx: NativeActorContext;
							isNew: boolean;
						},
					) => {
						const { ctx, isNew } = unwrapTsfnPayload(error, payload);
						const actorCtx = new NativeActorContextAdapter(
							ctx,
							createClient,
							schemaConfig,
						);
						try {
							await config.onMigrate(actorCtx, isNew);
						} finally {
							await actorCtx.dispose();
						}
					})
				: undefined,
		onSleep:
			typeof config.onSleep === "function"
				? wrapNativeCallback(async (
						error: unknown,
						payload: { ctx: NativeActorContext },
					) => {
						const { ctx } = unwrapTsfnPayload(error, payload);
						const actorCtx = new NativeActorContextAdapter(
							ctx,
							createClient,
							schemaConfig,
						);
						try {
							await config.onSleep(actorCtx);
						} finally {
							await actorCtx.dispose();
						}
					})
				: undefined,
		onDestroy:
			typeof config.onDestroy === "function"
				? wrapNativeCallback(async (
						error: unknown,
						payload: { ctx: NativeActorContext },
					) => {
						const { ctx } = unwrapTsfnPayload(error, payload);
						const actorCtx = new NativeActorContextAdapter(
							ctx,
							createClient,
							schemaConfig,
						);
						try {
							await config.onDestroy(actorCtx);
						} finally {
							await actorCtx.dispose();
						}
					})
				: undefined,
		onStateChange:
			typeof config.onStateChange === "function"
				? wrapNativeCallback(async (
						error: unknown,
						payload: {
							ctx: NativeActorContext;
							newState: Buffer;
						},
					) => {
						const { ctx, newState } = unwrapTsfnPayload(error, payload);
						const actorCtx = new NativeActorContextAdapter(
							ctx,
							createClient,
							schemaConfig,
						);
						try {
							await config.onStateChange(actorCtx, decodeValue(newState));
						} finally {
							await actorCtx.dispose();
						}
					})
				: undefined,
		onBeforeConnect:
			typeof config.onBeforeConnect === "function"
				? wrapNativeCallback(async (
						error: unknown,
						payload: {
							ctx: NativeActorContext;
							params: Buffer;
						},
					) => {
						const { ctx, params } = unwrapTsfnPayload(error, payload);
						const actorCtx = new NativeActorContextAdapter(
							ctx,
							createClient,
							schemaConfig,
						);
						try {
							await config.onBeforeConnect(
								actorCtx,
								validateConnParams(
									schemaConfig.connParamsSchema,
									decodeValue(params),
								),
							);
						} finally {
							await actorCtx.dispose();
						}
					})
				: undefined,
		onConnect:
			typeof config.onConnect === "function"
				? wrapNativeCallback(async (
						error: unknown,
						payload: {
							ctx: NativeActorContext;
							conn: NativeConnHandle;
						},
					) => {
						const { ctx, conn } = unwrapTsfnPayload(error, payload);
						const actorCtx = withConnContext(
							ctx,
							conn,
							createClient,
							schemaConfig,
						);
						try {
							await config.onConnect(
								actorCtx,
								new NativeConnAdapter(conn, schemaConfig),
							);
						} finally {
							await actorCtx.dispose();
						}
					})
				: undefined,
		onDisconnect:
			typeof config.onDisconnect === "function"
				? wrapNativeCallback(async (
						error: unknown,
						payload: {
							ctx: NativeActorContext;
							conn: NativeConnHandle;
						},
					) => {
						const { ctx, conn } = unwrapTsfnPayload(error, payload);
						const actorCtx = withConnContext(
							ctx,
							conn,
							createClient,
							schemaConfig,
						);
						try {
							await config.onDisconnect(
								actorCtx,
								new NativeConnAdapter(conn, schemaConfig),
							);
						} finally {
							await actorCtx.dispose();
						}
					})
				: undefined,
		onBeforeActionResponse:
			typeof config.onBeforeActionResponse === "function"
				? wrapNativeCallback(async (
						error: unknown,
						payload: {
							ctx: NativeActorContext;
							name: string;
							args: Buffer;
							output: Buffer;
						},
					) => {
						const { ctx, name, args, output } = unwrapTsfnPayload(
							error,
							payload,
						);
						const actorCtx = new NativeActorContextAdapter(
							ctx,
							createClient,
							schemaConfig,
						);
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
					})
				: undefined,
		onRequest: wrapNativeCallback(async (
			error: unknown,
			payload: {
				ctx: NativeActorContext;
				request: {
					method: string;
					uri: string;
					headers?: Record<string, string>;
					body?: Buffer;
				};
			},
		) => {
			try {
				const { ctx, request } = unwrapTsfnPayload(error, payload);
				const jsRequest = buildRequest(request);
				const actionResponse = await maybeHandleNativeActionRequest(
					ctx,
					jsRequest,
					createClient,
					actionHandlers,
					schemaConfig,
				);
				if (actionResponse) {
					return await toJsHttpResponse(actionResponse);
				}

				if (typeof config.onRequest !== "function") {
					return await toJsHttpResponse(new Response(null, { status: 404 }));
				}

				const requestCtx = Object.assign(
					new NativeActorContextAdapter(ctx, createClient, schemaConfig),
					{
						request: jsRequest,
					},
				);
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
		}),
		onWebSocket:
			typeof config.onWebSocket === "function"
				? wrapNativeCallback(async (
						error: unknown,
						payload: {
							ctx: NativeActorContext;
							ws: NativeWebSocket;
						},
					) => {
						const { ctx, ws } = unwrapTsfnPayload(error, payload);
						const actorCtx = new NativeActorContextAdapter(
							ctx,
							createClient,
							schemaConfig,
						);
						try {
							await config.onWebSocket(
								actorCtx,
								new NativeWebSocketAdapter(ws),
							);
						} finally {
							await actorCtx.dispose();
						}
					})
				: undefined,
		run: (() => {
			const run = getRunFunction(config.run);
			if (!run) {
				return undefined;
			}

			return wrapNativeCallback(async (
				error: unknown,
				payload: { ctx: NativeActorContext },
			) => {
				const { ctx } = unwrapTsfnPayload(error, payload);
				const actorCtx = new NativeActorContextAdapter(
					ctx,
					createClient,
					schemaConfig,
				);
				try {
					await run(actorCtx);
				} finally {
					await actorCtx.dispose();
				}
			});
			})(),
			actions: Object.fromEntries(
				Object.entries(actionHandlers).map(([name, handler]) => [
					name,
					wrapNativeCallback(async (
						error: unknown,
						payload: {
							ctx: NativeActorContext;
							conn: NativeConnHandle;
							args: Buffer;
						},
					) => {
						const { ctx, conn, args } = unwrapTsfnPayload(error, payload);
						const actorCtx = withConnContext(
							ctx,
							conn,
							createClient,
							schemaConfig,
						);
						try {
							return encodeValue(
								await handler(
									actorCtx,
									...validateActionArgs(
										schemaConfig.actionInputSchemas,
										name,
										decodeArgs(args),
									),
								),
							);
						} finally {
							await actorCtx.dispose();
						}
					}),
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
		registry.register(name, buildNativeFactory(bindings, config, definition));
	}

	return {
		registry,
		serveConfig: await buildServeConfig(config),
	};
}
