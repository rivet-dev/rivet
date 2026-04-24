import type {
	JsActorConfig,
	JsHttpResponse,
	JsServeConfig,
	ActorContext as NativeActorContext,
	NapiActorFactory as NativeActorFactory,
	CancellationToken as NativeCancellationToken,
	ConnHandle as NativeConnHandle,
	CoreRegistry as NativeCoreRegistry,
	Queue as NativeQueue,
	QueueMessage as NativeQueueMessage,
	Schedule as NativeSchedule,
	StateDeltaPayload as NativeStateDeltaPayload,
	WebSocket as NativeWebSocket,
} from "@rivetkit/rivetkit-napi";
import { VirtualWebSocket } from "@rivetkit/virtual-websocket";
import {
	ACTOR_CONTEXT_INTERNAL_SYMBOL,
	CONN_STATE_MANAGER_SYMBOL,
	getRunFunction,
	getRunInspectorConfig,
	type WorkflowInspectorConfig,
} from "@/actor/config";
import type { AnyActorDefinition } from "@/actor/definition";
import {
	decodeBridgeRivetError,
	encodeBridgeRivetError,
	type RivetErrorLike,
	forbiddenError,
	INTERNAL_ERROR_CODE,
	isRivetErrorLike,
	RivetError,
	toRivetError,
} from "@/actor/errors";
import { makePrefixedKey, removePrefixFromKey } from "@/actor/keys";
import {
	getEventCanSubscribe,
	getQueueCanPublish,
	hasSchemaConfigKey,
} from "@/actor/schema";
import {
	type AnyClient,
	type Client,
	createClientWithDriver,
} from "@/client/client";
import { convertRegistryConfigToClientConfig } from "@/client/config";
import {
	HEADER_CONN_PARAMS,
	HEADER_ENCODING,
} from "@/common/actor-router-consts";
import type * as protocol from "@/common/client-protocol";
import {
	CURRENT_VERSION as CLIENT_PROTOCOL_CURRENT_VERSION,
	HTTP_ACTION_REQUEST_VERSIONED,
	HTTP_ACTION_RESPONSE_VERSIONED,
	HTTP_QUEUE_SEND_REQUEST_VERSIONED,
	HTTP_QUEUE_SEND_RESPONSE_VERSIONED,
	HTTP_RESPONSE_ERROR_VERSIONED,
} from "@/common/client-protocol-versioned";
import {
	HttpActionRequestSchema,
	HttpActionResponseSchema,
	type HttpQueueSendRequest as HttpQueueSendRequestJson,
	HttpQueueSendRequestSchema,
	type HttpQueueSendResponse as HttpQueueSendResponseJson,
	HttpQueueSendResponseSchema,
	type HttpResponseError as HttpResponseErrorJson,
	HttpResponseErrorSchema,
} from "@/common/client-protocol-zod";
import type { AnyDatabaseProvider } from "@/common/database/config";
import { wrapJsNativeDatabase } from "@/common/database/native-database";
import type { Encoding } from "@/common/encoding";
import { decodeWorkflowHistoryTransport } from "@/common/inspector-transport";
import { deconstructError } from "@/common/utils";
import type {
	RivetCloseEvent,
	RivetEvent,
	RivetMessageEvent,
	UniversalWebSocket,
} from "@/common/websocket-interface";
import { RemoteEngineControlClient } from "@/engine-client/mod";
import type { Registry } from "@/registry";
import type { RegistryConfig } from "@/registry/config";
import {
	contentTypeForEncoding,
	decodeCborCompat,
	deserializeWithEncoding,
	encodeCborCompat,
	serializeWithEncoding,
} from "@/serde";
import { bufferToArrayBuffer, VERSION } from "@/utils";
import { logger } from "./log";
import {
	type NativeValidationConfig,
	validateActionArgs,
	validateConnParams,
	validateEventArgs,
	validateQueueBody,
	validateQueueComplete,
} from "./native-validation";

type NativeBindings = typeof import("@rivetkit/rivetkit-napi");
type NativeWebSocketEvent =
	| {
			kind: "message";
			data: string | Buffer;
			binary: boolean;
			messageIndex?: number;
	  }
	| {
			kind: "close";
			code: number;
			reason: string;
			wasClean: boolean;
	  };
type NativeWebSocketWithEvents = NativeWebSocket & {
	setEventCallback: (callback: (event: NativeWebSocketEvent) => void) => void;
};
const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();
type SerializeStateReason = "save" | "inspector";
type NativeOnStateChangeHandler = (
	ctx: NativeActorContextAdapter,
	state: unknown,
) => void | Promise<void>;
type NativePersistConnState = {
	state: unknown;
};
type NativePersistActorState = {
	state: unknown;
	isInOnStateChange: boolean;
	connStates: Map<string, NativePersistConnState>;
};
type NativeDestroyGate = {
	destroyCompletion?: Promise<void>;
	resolveDestroy?: () => void;
};
type NativeDatabaseClientState = {
	client: unknown;
	provider: Exclude<AnyDatabaseProvider, undefined>;
};
type NativeActorRuntimeState = {
	sql?: ReturnType<typeof wrapJsNativeDatabase>;
	databaseClient?: NativeDatabaseClientState;
	varsInitialized?: boolean;
	vars?: unknown;
	destroyGate?: NativeDestroyGate;
	persistState?: NativePersistActorState;
};

// Keep JS-only actor caches on the NAPI ActorContext runtime-state bag instead
// of actorId-keyed module globals so same-key recreates start from a fresh
// generation.
function getNativeRuntimeState(
	ctx: NativeActorContext,
): NativeActorRuntimeState {
	const runtimeState = callNativeSync(() =>
		ctx.runtimeState(),
	) as NativeActorRuntimeState;
	if (!runtimeState.destroyGate) {
		runtimeState.destroyGate = {};
	}
	if (!runtimeState.persistState) {
		runtimeState.persistState = {
			state: undefined,
			isInOnStateChange: false,
			connStates: new Map(),
		};
	}
	return runtimeState;
}

function getNativePersistState(ctx: NativeActorContext): NativePersistActorState {
	return getNativeRuntimeState(ctx).persistState!;
}

function isPromiseLike(value: unknown): value is PromiseLike<void> {
	return (
		typeof value === "object" &&
		value !== null &&
		"then" in value &&
		typeof value.then === "function"
	);
}

function getNativeConnPersistState(
	ctx: NativeActorContext,
	conn: NativeConnHandle,
): NativePersistConnState {
	const persistState = getNativePersistState(ctx);
	const connId = callNativeSync(() => conn.id());
	let connState = persistState.connStates.get(connId);
	if (!connState) {
		connState = {
			state: undefined,
		};
		persistState.connStates.set(connId, connState);
	}
	return connState;
}

function stateMutationReentrantError(): RivetError {
	return new RivetError(
		"actor",
		"state_mutation_reentrant",
		"State mutations are not allowed inside onStateChange.",
	);
}

function databaseNotConfiguredError(): RivetError {
	return new RivetError(
		"actor",
		"database_not_configured",
		"database is not configured for this actor",
		{ public: true },
	);
}

function stateNotEnabledError(): RivetError {
	return new RivetError(
		"actor",
		"state_not_enabled",
		"State not enabled. Must implement `createState` or `state` to use state. (https://www.rivet.dev/docs/actors/state/#initializing-state)",
		{ public: true },
	);
}

function nativeClientNotConfiguredError(): RivetError {
	return new RivetError(
		"native",
		"client_not_configured",
		"native actor client is not configured",
		{ public: true },
	);
}

function nativeEndpointNotConfiguredError(): RivetError {
	return new RivetError(
		"native",
		"endpoint_not_configured",
		"registry endpoint is required for native envoy startup",
		{ public: true },
	);
}

function getNativeDestroyGate(ctx: NativeActorContext) {
	return getNativeRuntimeState(ctx).destroyGate!;
}

function markNativeDestroyRequested(ctx: NativeActorContext) {
	const gate = getNativeDestroyGate(ctx);
	if (!gate.destroyCompletion) {
		gate.destroyCompletion = new Promise<void>((resolve) => {
			gate!.resolveDestroy = resolve;
		});
	}
}

function resolveNativeDestroy(ctx: NativeActorContext) {
	const gate = getNativeRuntimeState(ctx).destroyGate;
	if (!gate?.resolveDestroy) {
		return;
	}

	gate.resolveDestroy();
	gate.resolveDestroy = undefined;
	gate.destroyCompletion = undefined;
}

function closeNativeSqlDatabase(
	ctx: NativeActorContext,
): Promise<void> | undefined {
	const runtimeState = getNativeRuntimeState(ctx);
	const database = runtimeState.sql;
	if (!database) {
		return;
	}

	runtimeState.sql = undefined;
	return database.close();
}

async function closeNativeDatabaseClient(
	ctx: NativeActorContext,
	destroy: boolean,
): Promise<void> {
	void destroy;
	const runtimeState = getNativeRuntimeState(ctx);
	const entry = runtimeState.databaseClient;
	if (!entry) {
		return;
	}

	runtimeState.databaseClient = undefined;

	if (typeof entry.provider.onDestroy === "function") {
		await entry.provider.onDestroy(entry.client as never);
		return;
	}

	if (
		entry.client &&
		typeof entry.client === "object" &&
		"close" in entry.client &&
		typeof entry.client.close === "function"
	) {
		await entry.client.close();
	}
}

function getOrCreateNativeSqlDatabase(
	ctx: NativeActorContext,
): ReturnType<typeof wrapJsNativeDatabase> {
	const runtimeState = getNativeRuntimeState(ctx);
	const cachedDatabase = runtimeState.sql;
	if (cachedDatabase) {
		return cachedDatabase;
	}

	const database = wrapJsNativeDatabase(callNativeSync(() => ctx.sql()));
	runtimeState.sql = database;
	return database;
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

type NativeKvValueType = "text" | "arrayBuffer" | "binary";
type NativeKvKeyType = "text" | "binary";

type NativeKvValueTypeMap = {
	text: string;
	arrayBuffer: ArrayBuffer;
	binary: Uint8Array;
};

type NativeKvKeyTypeMap = {
	text: string;
	binary: Uint8Array;
};

type NativeKvValueOptions<T extends NativeKvValueType = "text"> = {
	type?: T;
};

type NativeKvListOptions<
	T extends NativeKvValueType = "text",
	K extends NativeKvKeyType = "text",
> = NativeKvValueOptions<T> & {
	keyType?: K;
	reverse?: boolean;
	limit?: number;
};

function decodeNativeKvKey<K extends NativeKvKeyType = "text">(
	key: Uint8Array,
	keyType?: K,
): NativeKvKeyTypeMap[K] {
	const resolvedKeyType = keyType ?? "text";
	switch (resolvedKeyType) {
		case "text":
			return textDecoder.decode(key) as NativeKvKeyTypeMap[K];
		case "binary":
			return key as NativeKvKeyTypeMap[K];
		default:
			throw new TypeError("Invalid kv key type");
	}
}

function encodeNativeKvUserKey<K extends NativeKvKeyType = NativeKvKeyType>(
	key: NativeKvKeyTypeMap[K],
	keyType?: K,
): Uint8Array {
	if (key instanceof Uint8Array) {
		return key;
	}
	const resolvedKeyType = keyType ?? "text";
	if (resolvedKeyType === "binary") {
		throw new TypeError("Expected a Uint8Array when keyType is binary");
	}
	return textEncoder.encode(key);
}

function decodeNativeKvValue<T extends NativeKvValueType = "text">(
	value: Uint8Array,
	options?: NativeKvValueOptions<T>,
): NativeKvValueTypeMap[T] {
	const type = options?.type ?? "text";
	switch (type) {
		case "text":
			return textDecoder.decode(value) as NativeKvValueTypeMap[T];
		case "arrayBuffer": {
			const copy = new Uint8Array(value.byteLength);
			copy.set(value);
			return copy.buffer as NativeKvValueTypeMap[T];
		}
		case "binary":
			return value as NativeKvValueTypeMap[T];
		default:
			throw new TypeError("Invalid kv value type");
	}
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

	return decodeCborCompat(Buffer.from(value));
}

function encodeValue(value: unknown): Buffer {
	return Buffer.from(encodeCborCompat(value));
}

function unwrapTsfnPayload<T>(error: unknown, payload: T): T {
	if (error !== null && error !== undefined) {
		throw error;
	}

	return payload;
}

function normalizeNativeBridgeError(error: unknown): unknown {
	const promoteKnownBridgeError = (value: unknown): unknown => {
		if (!isRivetErrorLike(value)) {
			return value;
		}

		if (
			value.group === "auth" &&
			value.code === "forbidden" &&
			(!value.public || value.statusCode === 500)
		) {
			return new RivetError(value.group, value.code, value.message, {
				public: true,
				statusCode: 403,
				metadata: value.metadata,
				cause: value instanceof Error ? value.cause : undefined,
			});
		}

		if (
			value.group === "actor" &&
			value.code === "action_not_found" &&
			(!value.public || value.statusCode === 500)
		) {
			return new RivetError(value.group, value.code, value.message, {
				public: true,
				statusCode: 404,
				metadata: value.metadata,
				cause: value instanceof Error ? value.cause : undefined,
			});
		}

		if (
			value.group === "actor" &&
			value.code === "action_timed_out" &&
			(!value.public || value.statusCode === 500)
		) {
			return new RivetError(value.group, value.code, value.message, {
				public: true,
				statusCode: 408,
				metadata: value.metadata,
				cause: value instanceof Error ? value.cause : undefined,
			});
		}

		if (
			value.group === "actor" &&
			value.code === "aborted" &&
			(!value.public || value.statusCode === 500)
		) {
			return new RivetError(value.group, value.code, value.message, {
				public: true,
				statusCode: 400,
				metadata: value.metadata,
				cause: value instanceof Error ? value.cause : undefined,
			});
		}

		if (
			value.group === "message" &&
			(value.code === "incoming_too_long" ||
				value.code === "outgoing_too_long") &&
			(!value.public || value.statusCode === 500)
		) {
			return new RivetError(value.group, value.code, value.message, {
				public: true,
				statusCode: 400,
				metadata: value.metadata,
				cause: value instanceof Error ? value.cause : undefined,
			});
		}

		if (
			value.group === "queue" &&
			[
				"full",
				"message_too_large",
				"message_invalid",
				"invalid_payload",
				"invalid_completion_payload",
				"already_completed",
				"previous_message_not_completed",
				"complete_not_configured",
				"timed_out",
			].includes(value.code) &&
			(!value.public || value.statusCode === 500)
		) {
			return new RivetError(value.group, value.code, value.message, {
				public: true,
				statusCode: 400,
				metadata: value.metadata,
				cause: value instanceof Error ? value.cause : undefined,
			});
		}

		return value;
	};

	if (typeof error === "string") {
		return promoteKnownBridgeError(decodeBridgeRivetError(error) ?? error);
	}

	if (error instanceof Error) {
		const bridged = decodeBridgeRivetError(error.message);
		if (bridged) {
			return promoteKnownBridgeError(bridged);
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
			return promoteKnownBridgeError(bridged);
		}
	}

	return promoteKnownBridgeError(error);
}

function isStructuredBridgeError(
	error: unknown,
): error is RivetError | RivetErrorLike {
	if (error instanceof RivetError) {
		return true;
	}

	return (
		isRivetErrorLike(error) &&
		"__type" in error &&
		(error.__type === "RivetError" || error.__type === "ActorError")
	);
}

function encodeNativeCallbackError(error: unknown): Error {
	const structuredError = isStructuredBridgeError(error)
		? error
		: deconstructError(error, logger(), {
				bridge: "native_callback",
			});

	const bridgeError = new Error(encodeBridgeRivetError(structuredError), {
		cause: error instanceof Error ? error : undefined,
	});
	return Object.assign(bridgeError, {
		group: structuredError.group,
		code: structuredError.code,
		metadata: structuredError.metadata,
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

function actorAbortedError(): Error & { group: string; code: string } {
	return Object.assign(new Error("Actor aborted"), {
		group: "actor",
		code: "aborted",
	});
}

type NativeWorkflowInspectorConfig = WorkflowInspectorConfig<ArrayBuffer> & {
	getState?: () => Promise<unknown> | unknown;
};

function isClosedTaskRegistrationError(error: unknown): boolean {
	const metadata = error instanceof RivetError ? error.metadata : undefined;
	const metadataError =
		metadata && typeof metadata === "object" && "error" in metadata
			? metadata.error
			: undefined;
	return (
		error instanceof RivetError &&
		error.group === "core" &&
		error.code === INTERNAL_ERROR_CODE &&
		typeof metadataError === "string" &&
		/actor task registration is (closed|not configured)/.test(metadataError)
	);
}

async function createNativeCancellationToken(signal?: AbortSignal): Promise<{
	token?: NativeCancellationToken;
	cleanup?: () => void;
}> {
	if (!signal) {
		return {};
	}

	const bindings = await loadNativeBindings();
	const token = new bindings.CancellationToken();

	if (signal.aborted) {
		token.cancel();
		return { token };
	}

	const abort = () => token.cancel();
	signal.addEventListener("abort", abort, { once: true });
	return {
		token,
		cleanup: () => signal.removeEventListener("abort", abort),
	};
}

function decodeWorkflowCbor(data: ArrayBuffer | null): unknown | null {
	if (data === null) {
		return null;
	}

	try {
		return decodeCborCompat(new Uint8Array(data));
	} catch {
		return null;
	}
}

function serializeWorkflowLocation(
	location: ReturnType<
		typeof decodeWorkflowHistoryTransport
	>["entries"][number]["location"],
): Array<
	| { tag: "WorkflowNameIndex"; val: number }
	| {
			tag: "WorkflowLoopIterationMarker";
			val: { loop: number; iteration: number };
	  }
> {
	return location.map((segment) => {
		if (segment.tag === "WorkflowNameIndex") {
			return {
				tag: segment.tag,
				val: segment.val,
			};
		}

		return {
			tag: segment.tag,
			val: {
				loop: segment.val.loop,
				iteration: segment.val.iteration,
			},
		};
	});
}

function serializeWorkflowBranches(
	branches: ReadonlyMap<
		string,
		ReturnType<
			typeof decodeWorkflowHistoryTransport
		>["entries"][number]["kind"] extends infer T
			? T extends { tag: "WorkflowJoinEntry"; val: { branches: infer B } }
				? B extends ReadonlyMap<string, infer V>
					? V
					: never
				: T extends {
							tag: "WorkflowRaceEntry";
							val: { branches: infer B };
						}
					? B extends ReadonlyMap<string, infer V>
						? V
						: never
					: never
			: never
	>,
): Record<
	string,
	{ status: string; output: unknown | null; error: string | null }
> {
	return Object.fromEntries(
		Array.from(branches.entries()).map(([name, branch]) => [
			name,
			{
				status: branch.status,
				output: decodeWorkflowCbor(branch.output),
				error: branch.error,
			},
		]),
	);
}

function serializeWorkflowEntryKind(
	kind: ReturnType<
		typeof decodeWorkflowHistoryTransport
	>["entries"][number]["kind"],
):
	| {
			tag: "WorkflowStepEntry";
			val: { output: unknown | null; error: string | null };
	  }
	| {
			tag: "WorkflowLoopEntry";
			val: {
				state: unknown | null;
				iteration: number;
				output: unknown | null;
			};
	  }
	| { tag: "WorkflowSleepEntry"; val: { deadline: number; state: string } }
	| {
			tag: "WorkflowMessageEntry";
			val: { name: string; messageData: unknown | null };
	  }
	| { tag: "WorkflowRollbackCheckpointEntry"; val: { name: string } }
	| {
			tag: "WorkflowJoinEntry";
			val: {
				branches: Record<
					string,
					{
						status: string;
						output: unknown | null;
						error: string | null;
					}
				>;
			};
	  }
	| {
			tag: "WorkflowRaceEntry";
			val: {
				winner: string | null;
				branches: Record<
					string,
					{
						status: string;
						output: unknown | null;
						error: string | null;
					}
				>;
			};
	  }
	| {
			tag: "WorkflowRemovedEntry";
			val: { originalType: string; originalName: string | null };
	  } {
	switch (kind.tag) {
		case "WorkflowStepEntry":
			return {
				tag: kind.tag,
				val: {
					output: decodeWorkflowCbor(kind.val.output),
					error: kind.val.error,
				},
			};
		case "WorkflowLoopEntry":
			return {
				tag: kind.tag,
				val: {
					state: decodeWorkflowCbor(kind.val.state),
					iteration: kind.val.iteration,
					output: decodeWorkflowCbor(kind.val.output),
				},
			};
		case "WorkflowSleepEntry":
			return {
				tag: kind.tag,
				val: {
					deadline: Number(kind.val.deadline),
					state: kind.val.state,
				},
			};
		case "WorkflowMessageEntry":
			return {
				tag: kind.tag,
				val: {
					name: kind.val.name,
					messageData: decodeWorkflowCbor(kind.val.messageData),
				},
			};
		case "WorkflowRollbackCheckpointEntry":
			return {
				tag: kind.tag,
				val: {
					name: kind.val.name,
				},
			};
		case "WorkflowJoinEntry":
			return {
				tag: kind.tag,
				val: {
					branches: serializeWorkflowBranches(kind.val.branches),
				},
			};
		case "WorkflowRaceEntry":
			return {
				tag: kind.tag,
				val: {
					winner: kind.val.winner,
					branches: serializeWorkflowBranches(kind.val.branches),
				},
			};
		case "WorkflowRemovedEntry":
			return {
				tag: kind.tag,
				val: {
					originalType: kind.val.originalType,
					originalName: kind.val.originalName,
				},
			};
	}
}

function serializeWorkflowHistoryForJson(data: ArrayBuffer | null): {
	nameRegistry: string[];
	entries: Array<{
		id: string;
		location: Array<
			| { tag: "WorkflowNameIndex"; val: number }
			| {
					tag: "WorkflowLoopIterationMarker";
					val: { loop: number; iteration: number };
			  }
		>;
		kind: ReturnType<typeof serializeWorkflowEntryKind>;
	}>;
	entryMetadata: Record<
		string,
		{
			status: string;
			error: string | null;
			attempts: number;
			lastAttemptAt: number;
			createdAt: number;
			completedAt: number | null;
			rollbackCompletedAt: number | null;
			rollbackError: string | null;
		}
	>;
} | null {
	if (data === null) {
		return null;
	}

	const history = decodeWorkflowHistoryTransport(data);

	return {
		nameRegistry: [...history.nameRegistry],
		entries: history.entries.map((entry) => ({
			id: entry.id,
			location: serializeWorkflowLocation(entry.location),
			kind: serializeWorkflowEntryKind(entry.kind),
		})),
		entryMetadata: Object.fromEntries(
			Array.from(history.entryMetadata.entries()).map(
				([entryId, meta]) => [
					entryId,
					{
						status: meta.status,
						error: meta.error,
						attempts: meta.attempts,
						lastAttemptAt: Number(meta.lastAttemptAt),
						createdAt: Number(meta.createdAt),
						completedAt:
							meta.completedAt === null
								? null
								: Number(meta.completedAt),
						rollbackCompletedAt:
							meta.rollbackCompletedAt === null
								? null
								: Number(meta.rollbackCompletedAt),
						rollbackError: meta.rollbackError,
					},
				],
			),
		),
	};
}

function toHttpJsonCompatible<T>(value: T): T {
	return JSON.parse(
		JSON.stringify(value, (_key, nestedValue) =>
			typeof nestedValue === "bigint"
				? Number(nestedValue)
				: nestedValue instanceof Uint8Array
					? Array.from(nestedValue)
					: nestedValue,
		),
	) as T;
}

function jsonSafe<T>(value: T): T {
	return toHttpJsonCompatible(value);
}

function normalizeSqlitePropertyBindings(
	properties: Record<string, unknown>,
): Record<string, unknown> {
	const normalized: Record<string, unknown> = {};
	for (const [key, value] of Object.entries(properties)) {
		if (/^[:@$]/.test(key)) {
			normalized[key] = value;
			continue;
		}

		normalized[`:${key}`] = value;
		normalized[`@${key}`] = value;
		normalized[`$${key}`] = value;
	}
	return normalized;
}

function queryRows(result: unknown): Array<Record<string, unknown>> {
	if (Array.isArray(result)) {
		return result as Array<Record<string, unknown>>;
	}
	if (
		result &&
		typeof result === "object" &&
		"columns" in result &&
		"rows" in result &&
		Array.isArray((result as { columns: unknown }).columns) &&
		Array.isArray((result as { rows: unknown }).rows)
	) {
		const columns = (result as { columns: string[] }).columns;
		return ((result as { rows: unknown[][] }).rows ?? []).map((row) =>
			Object.fromEntries(
				columns.map((column, index) => [column, row[index]]),
			),
		);
	}
	return [];
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
	return Array.isArray(decoded)
		? decoded
		: decoded === undefined
			? []
			: [decoded];
}

function createWriteThroughProxy<T>(
	value: T,
	commit: (next: T) => void,
	beforeChange?: () => void,
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
				beforeChange?.();
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
				beforeChange?.();
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
	segments: Array<{
		kind: string;
		stringValue?: string;
		numberValue?: number;
	}>,
): string[] {
	return segments.map((segment) =>
		segment.kind === "number"
			? String(segment.numberValue ?? 0)
			: (segment.stringValue ?? ""),
	);
}

class NativeConnAdapter {
	#conn: NativeConnHandle;
	#schemas: NativeValidationConfig;
	#ctx?: NativeActorContext;
	#queueHibernationRemoval?: (connId: string) => void;

	constructor(
		conn: NativeConnHandle,
		schemas: NativeValidationConfig = {},
		ctx?: NativeActorContext,
		queueHibernationRemoval?: (connId: string) => void,
	) {
		this.#conn = conn;
		this.#schemas = schemas;
		this.#ctx = ctx;
		this.#queueHibernationRemoval = queueHibernationRemoval;
		(
			this as NativeConnAdapter & {
				[CONN_STATE_MANAGER_SYMBOL]?: unknown;
			}
		)[CONN_STATE_MANAGER_SYMBOL] = {
			stateEnabled: true,
			get state() {
				return thisConn.state;
			},
		};
		const thisConn = this;
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
		const nextState = this.#readState();
		return createWriteThroughProxy(
			nextState,
			(nextValue) => {
				this.#writeState(nextValue, { writeNative: true });
			},
		);
	}

	set state(value: unknown) {
		this.#writeState(value, { writeNative: true });
	}

	initializeState(value: unknown): void {
		this.#writeState(value, { writeNative: false });
	}

	get isHibernatable(): boolean {
		return callNativeSync(() => this.#conn.isHibernatable());
	}

	send(name: string, ...args: unknown[]): void {
		const validatedArgs = validateEventArgs(
			this.#schemas.events,
			name,
			args,
		);
		callNativeSync(() => this.#conn.send(name, encodeValue(validatedArgs)));
	}

	async disconnect(reason?: string): Promise<void> {
		const connId = this.id;
		await callNative(() => this.#conn.disconnect(reason));
		if (this.isHibernatable) {
			this.#queueHibernationRemoval?.(connId);
		}
	}

	#readState(): unknown {
		if (!this.#ctx) {
			return decodeValue(this.#conn.state());
		}

		const connState = getNativeConnPersistState(this.#ctx, this.#conn);
		if (connState.state === undefined) {
			connState.state = decodeValue(this.#conn.state());
		}
		return connState.state;
	}

	#writeState(
		value: unknown,
		options: {
			writeNative: boolean;
		},
	): void {
		const encoded = encodeValue(value);
		if (!this.#ctx) {
			this.#conn.setState(encoded);
			return;
		}

		const connState = getNativeConnPersistState(this.#ctx, this.#conn);
		connState.state = value;
		if (options.writeNative) {
			this.#conn.setState(encoded);
		}
	}
}

class NativeScheduleAdapter {
	#schedule: NativeSchedule;

	constructor(schedule: NativeSchedule) {
		this.#schedule = schedule;
	}

	async after(
		duration: number,
		action: string,
		...args: unknown[]
	): Promise<void> {
		callNativeSync(() =>
			this.#schedule.after(duration, action, encodeValue(args)),
		);
	}

	async at(
		timestamp: number,
		action: string,
		...args: unknown[]
	): Promise<void> {
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

	async get<T extends NativeKvValueType = "text">(
		key: string | Uint8Array,
		options?: NativeKvValueOptions<T>,
	): Promise<NativeKvValueTypeMap[T] | null> {
		const value = await callNative(() =>
			this.#kv.get(
				Buffer.from(makePrefixedKey(encodeNativeKvUserKey(key))),
			),
		);
		return value
			? decodeNativeKvValue(new Uint8Array(value), options)
			: null;
	}

	async put(
		key: string | Uint8Array,
		value: string | Uint8Array | ArrayBuffer,
		_options?: NativeKvValueOptions,
	): Promise<void> {
		await callNative(() =>
			this.#kv.put(
				Buffer.from(makePrefixedKey(encodeNativeKvUserKey(key))),
				toBuffer(value),
			),
		);
	}

	async delete(key: string | Uint8Array): Promise<void> {
		await callNative(() =>
			this.#kv.delete(
				Buffer.from(makePrefixedKey(encodeNativeKvUserKey(key))),
			),
		);
	}

	async deleteRange(
		start: string | Uint8Array,
		end: string | Uint8Array,
	): Promise<void> {
		await callNative(() =>
			this.#kv.deleteRange(
				Buffer.from(makePrefixedKey(encodeNativeKvUserKey(start))),
				Buffer.from(makePrefixedKey(encodeNativeKvUserKey(end))),
			),
		);
	}

	async rawDeleteRange(start: Uint8Array, end: Uint8Array): Promise<void> {
		await callNative(() =>
			this.#kv.deleteRange(Buffer.from(start), Buffer.from(end)),
		);
	}

	async listPrefix<
		T extends NativeKvValueType = "text",
		K extends NativeKvKeyType = "text",
	>(
		prefix: string | Uint8Array,
		options?: NativeKvListOptions<T, K>,
	): Promise<Array<[NativeKvKeyTypeMap[K], NativeKvValueTypeMap[T]]>> {
		const entries = await callNative(() =>
			this.#kv.listPrefix(
				Buffer.from(
					makePrefixedKey(
						encodeNativeKvUserKey(
							prefix as NativeKvKeyTypeMap[K],
							options?.keyType,
						),
					),
				),
				{
					reverse: options?.reverse,
					limit: options?.limit,
				},
			),
		);
		return entries.map((entry) => [
			decodeNativeKvKey(
				removePrefixFromKey(new Uint8Array(entry.key)),
				options?.keyType,
			),
			decodeNativeKvValue(new Uint8Array(entry.value), options),
		]);
	}

	async rawListPrefix(
		prefix: Uint8Array,
	): Promise<Array<[Uint8Array, Uint8Array]>> {
		const entries = await callNative(() =>
			this.#kv.listPrefix(Buffer.from(prefix), {}),
		);
		return entries.map((entry) => [
			new Uint8Array(entry.key),
			new Uint8Array(entry.value),
		]);
	}

	async listRange<
		T extends NativeKvValueType = "text",
		K extends NativeKvKeyType = "text",
	>(
		start: string | Uint8Array,
		end: string | Uint8Array,
		options?: NativeKvListOptions<T, K>,
	): Promise<Array<[NativeKvKeyTypeMap[K], NativeKvValueTypeMap[T]]>> {
		const entries = await callNative(() =>
			this.#kv.listRange(
				Buffer.from(
					makePrefixedKey(
						encodeNativeKvUserKey(
							start as NativeKvKeyTypeMap[K],
							options?.keyType,
						),
					),
				),
				Buffer.from(
					makePrefixedKey(
						encodeNativeKvUserKey(
							end as NativeKvKeyTypeMap[K],
							options?.keyType,
						),
					),
				),
				{
					reverse: options?.reverse,
					limit: options?.limit,
				},
			),
		);
		return entries.map((entry) => [
			decodeNativeKvKey(
				removePrefixFromKey(new Uint8Array(entry.key)),
				options?.keyType,
			),
			decodeNativeKvValue(new Uint8Array(entry.value), options),
		]);
	}

	async list<
		T extends NativeKvValueType = "text",
		K extends NativeKvKeyType = "text",
	>(
		prefix: string | Uint8Array,
		options?: NativeKvListOptions<T, K>,
	): Promise<Array<[NativeKvKeyTypeMap[K], NativeKvValueTypeMap[T]]>> {
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
								: encodeValue(
										validateQueueComplete(
											schemas,
											name,
											response,
										),
									),
						),
					)
			: undefined,
	};
}

class NativeQueueAdapter {
	#queue: NativeQueue;
	#schemas: NativeValidationConfig["queues"];
	#pendingCompletableMessageIds = new Set<string>();

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
		signal?: AbortSignal;
		completable?: boolean;
	}) {
		const messages = await this.nextBatch({
			names: options?.names,
			count: 1,
			timeout: options?.timeout,
			signal: options?.signal,
			completable: options?.completable,
		});
		return messages[0];
	}

	async nextBatch(options?: {
		names?: readonly string[];
		count?: number;
		timeout?: number;
		signal?: AbortSignal;
		completable?: boolean;
	}) {
		const completable = options?.completable === true;
		if (this.#pendingCompletableMessageIds.size > 0) {
			throw new RivetError(
				"queue",
				"previous_message_not_completed",
				"Previous completable queue message is not completed. Call `message.complete(...)` before receiving the next message.",
				{
					public: true,
					statusCode: 400,
				},
			);
		}

		const { token, cleanup } = await createNativeCancellationToken(
			options?.signal,
		);

		try {
			const messages = await callNative(() =>
				this.#queue.nextBatch(
					{
						names: this.#normalizeNames(options?.names),
						count: options?.count,
						timeoutMs: options?.timeout,
						completable,
					},
					token,
				),
			);
			const wrapped = messages.map((message) =>
				wrapQueueMessage(message, this.#schemas),
			);
			return completable
				? wrapped.map((message) =>
						this.#makeCompletableMessage(message),
					)
				: wrapped;
		} finally {
			cleanup?.();
		}
	}

	async waitForNames(
		names: readonly string[],
		options?: {
			timeout?: number;
			signal?: AbortSignal;
			completable?: boolean;
		},
	) {
		const { token, cleanup } = await createNativeCancellationToken(
			options?.signal,
		);

		try {
			return wrapQueueMessage(
				await callNative(() =>
					this.#queue.waitForNames(
						[...names],
						{
							timeoutMs: options?.timeout,
							completable: options?.completable,
						},
						token,
					),
				),
				this.#schemas,
			);
		} finally {
			cleanup?.();
		}
	}

	async waitForNamesAvailable(
		names: readonly string[],
		options?: {
			timeout?: number;
			signal?: AbortSignal;
		},
	) {
		if (!options?.signal) {
			await callNative(() =>
				this.#queue.waitForNamesAvailable([...names], {
					timeoutMs: options?.timeout,
				}),
			);
			return;
		}

		const deadline =
			options.timeout === undefined
				? undefined
				: Date.now() + options.timeout;

		for (;;) {
			if (options.signal.aborted) {
				throw actorAbortedError();
			}

			const remainingTimeout =
				deadline === undefined
					? undefined
					: Math.max(0, deadline - Date.now());
			const sliceTimeout =
				remainingTimeout === undefined
					? 100
					: Math.min(remainingTimeout, 100);

			try {
				await callNative(() =>
					this.#queue.waitForNamesAvailable([...names], {
						timeoutMs: sliceTimeout,
					}),
				);
				return;
			} catch (error) {
				if (
					(error as { group?: string; code?: string }).group ===
						"queue" &&
					(error as { group?: string; code?: string }).code ===
						"timed_out"
				) {
					if (
						remainingTimeout === undefined ||
						remainingTimeout > 100
					) {
						continue;
					}
				}
				throw error;
			}
		}
	}

	async enqueueAndWait(
		name: string,
		body: unknown,
		options?: {
			timeout?: number;
			signal?: AbortSignal;
		},
	) {
		const validatedBody = validateQueueBody(this.#schemas, name, body);
		const { token, cleanup } = await createNativeCancellationToken(
			options?.signal,
		);

		try {
			const response = await callNative(() =>
				this.#queue.enqueueAndWait(
					name,
					encodeValue(validatedBody),
					{
						timeoutMs: options?.timeout,
					},
					token,
				),
			);
			return response === undefined || response === null
				? undefined
				: validateQueueComplete(
						this.#schemas,
						name,
						decodeValue(response),
					);
		} finally {
			cleanup?.();
		}
	}

	async tryNext(options?: {
		names?: readonly string[];
		completable?: boolean;
	}) {
		const messages = await this.tryNextBatch({
			names: options?.names,
			count: 1,
			completable: options?.completable,
		});
		return messages[0];
	}

	async tryNextBatch(options?: {
		names?: readonly string[];
		count?: number;
		completable?: boolean;
	}) {
		if (options?.completable) {
			return await this.nextBatch({
				names: options.names,
				count: options.count,
				timeout: 0,
				completable: true,
			});
		}

		const messages = callNativeSync(() =>
			this.#queue.tryNextBatch({
				names: this.#normalizeNames(options?.names),
				count: options?.count,
				completable: false,
			}),
		);
		return messages.map((message) =>
			wrapQueueMessage(message, this.#schemas),
		);
	}

	async *iter(options?: {
		names?: readonly string[];
		signal?: AbortSignal;
		completable?: boolean;
	}): AsyncIterableIterator<
		NonNullable<Awaited<ReturnType<NativeQueueAdapter["next"]>>>
	> {
		for (;;) {
			try {
				const message = await this.next(options);
				if (!message) {
					continue;
				}
				yield message;
			} catch (error) {
				if (
					isRivetErrorLike(error) &&
					error.group === "actor" &&
					error.code === "aborted"
				) {
					return;
				}
				throw error;
			}
		}
	}

	#normalizeNames(
		names: readonly string[] | undefined,
	): string[] | undefined {
		if (!names || names.length === 0) {
			return undefined;
		}
		return [...new Set(names)];
	}

	#makeCompletableMessage(
		message: Awaited<ReturnType<typeof wrapQueueMessage>>,
	) {
		const messageId = message.id.toString();
		this.#pendingCompletableMessageIds.add(messageId);
		let completed = false;

		return {
			...message,
			complete: async (response?: unknown) => {
				if (typeof message.complete !== "function") {
					throw new RivetError(
						"queue",
						"complete_not_configured",
						`Queue '${message.name}' does not support completion responses.`,
						{
							public: true,
							statusCode: 400,
							metadata: { name: message.name },
						},
					);
				}
				if (completed) {
					throw new RivetError(
						"queue",
						"already_completed",
						"Queue message was already completed.",
						{
							public: true,
							statusCode: 400,
						},
					);
				}

				try {
					await message.complete(response);
					completed = true;
					this.#pendingCompletableMessageIds.delete(messageId);
				} catch (error) {
					throw error;
				}
			},
		};
	}
}

class NativeWebSocketAdapter {
	#ws: NativeWebSocketWithEvents;
	#virtual: VirtualWebSocket;
	#readyState: 0 | 1 | 2 | 3 = VirtualWebSocket.OPEN;

	constructor(ws: NativeWebSocket) {
		this.#ws = ws as NativeWebSocketWithEvents;
		this.#virtual = new VirtualWebSocket({
			getReadyState: () => this.#readyState,
			onSend: (data) => {
				if (typeof data === "string") {
					callNativeSync(() =>
						this.#ws.send(Buffer.from(data), false),
					);
					return;
				}

				const buffer = ArrayBuffer.isView(data)
					? Buffer.from(data.buffer, data.byteOffset, data.byteLength)
					: Buffer.from(data as ArrayBufferLike);
				callNativeSync(() => this.#ws.send(buffer, true));
			},
			onClose: (code, reason) => {
				this.#readyState = VirtualWebSocket.CLOSING;
				void callNative(() => this.#ws.close(code, reason));
			},
		});
		this.#ws.setEventCallback((event) => {
			if (event.kind === "message") {
				this.#virtual.triggerMessage(
					event.binary
						? bufferToArrayBuffer(event.data as Buffer)
						: event.data,
					event.messageIndex,
				);
				return;
			}

			this.#readyState = VirtualWebSocket.CLOSED;
			this.#virtual.triggerClose(
				event.code,
				event.reason,
				event.wasClean,
			);
		});
	}

	get readyState() {
		return this.#virtual.readyState;
	}

	get CONNECTING() {
		return this.#virtual.CONNECTING;
	}

	get OPEN() {
		return this.#virtual.OPEN;
	}

	get CLOSING() {
		return this.#virtual.CLOSING;
	}

	get CLOSED() {
		return this.#virtual.CLOSED;
	}

	get binaryType() {
		return this.#virtual.binaryType;
	}

	set binaryType(value: "arraybuffer" | "blob") {
		this.#virtual.binaryType = value;
	}

	get bufferedAmount() {
		return this.#virtual.bufferedAmount;
	}

	get extensions() {
		return this.#virtual.extensions;
	}

	get protocol() {
		return this.#virtual.protocol;
	}

	get url() {
		return this.#virtual.url;
	}

	get onopen() {
		return this.#virtual.onopen;
	}

	set onopen(value) {
		this.#virtual.onopen = value;
	}

	get onclose() {
		return this.#virtual.onclose;
	}

	set onclose(value) {
		this.#virtual.onclose = value;
	}

	get onerror() {
		return this.#virtual.onerror;
	}

	set onerror(value) {
		this.#virtual.onerror = value;
	}

	get onmessage() {
		return this.#virtual.onmessage;
	}

	set onmessage(value) {
		this.#virtual.onmessage = value;
	}

	send(data: string | ArrayBuffer | ArrayBufferView): void {
		this.#virtual.send(data);
	}

	close(code?: number, reason?: string): void {
		this.#virtual.close(code, reason);
	}

	addEventListener(
		type: string,
		listener: (event: any) => void | Promise<void>,
	): void {
		this.#virtual.addEventListener(type, listener);
	}

	removeEventListener(
		type: string,
		listener: (event: any) => void | Promise<void>,
	): void {
		this.#virtual.removeEventListener(type, listener);
	}

	dispatchEvent(event: {
		type: string;
		target?: unknown;
		currentTarget?: unknown;
	}): boolean {
		return this.#virtual.dispatchEvent(event);
	}
}

type TrackedWebSocketListener = (event: any) => void | Promise<void>;

class TrackedNativeWebSocketAdapter implements UniversalWebSocket {
	#ctx: NativeActorContextAdapter;
	#inner: UniversalWebSocket;
	#listeners = new Map<string, TrackedWebSocketListener[]>();
	#onopen: ((event: RivetEvent) => void | Promise<void>) | null = null;
	#onclose: ((event: RivetCloseEvent) => void | Promise<void>) | null = null;
	#onerror: ((event: RivetEvent) => void | Promise<void>) | null = null;
	#onmessage: ((event: RivetMessageEvent) => void | Promise<void>) | null =
		null;

	constructor(ctx: NativeActorContextAdapter, inner: UniversalWebSocket) {
		this.#ctx = ctx;
		this.#inner = inner;

		inner.addEventListener("open", (event) => {
			this.#dispatch("open", this.#createEvent("open", event));
		});
		inner.addEventListener("message", (event) => {
			this.#dispatch("message", this.#createEvent("message", event));
		});
		inner.addEventListener("close", (event) => {
			this.#dispatch("close", this.#createEvent("close", event));
		});
		inner.addEventListener("error", (event) => {
			this.#dispatch("error", this.#createEvent("error", event));
		});
	}

	get CONNECTING(): 0 {
		return this.#inner.CONNECTING;
	}

	get OPEN(): 1 {
		return this.#inner.OPEN;
	}

	get CLOSING(): 2 {
		return this.#inner.CLOSING;
	}

	get CLOSED(): 3 {
		return this.#inner.CLOSED;
	}

	get readyState(): 0 | 1 | 2 | 3 {
		return this.#inner.readyState;
	}

	get binaryType(): "arraybuffer" | "blob" {
		return this.#inner.binaryType;
	}

	set binaryType(value: "arraybuffer" | "blob") {
		this.#inner.binaryType = value;
	}

	get bufferedAmount(): number {
		return this.#inner.bufferedAmount;
	}

	get extensions(): string {
		return this.#inner.extensions;
	}

	get protocol(): string {
		return this.#inner.protocol;
	}

	get url(): string {
		return this.#inner.url;
	}

	send(data: string | ArrayBufferLike | Blob | ArrayBufferView): void {
		this.#inner.send(data);
	}

	close(code?: number, reason?: string): void {
		this.#inner.close(code, reason);
	}

	addEventListener(type: string, listener: TrackedWebSocketListener): void {
		if (!this.#listeners.has(type)) {
			this.#listeners.set(type, []);
		}
		this.#listeners.get(type)!.push(listener);
	}

	removeEventListener(
		type: string,
		listener: TrackedWebSocketListener,
	): void {
		const listeners = this.#listeners.get(type);
		if (!listeners) {
			return;
		}

		const index = listeners.indexOf(listener);
		if (index !== -1) {
			listeners.splice(index, 1);
		}
	}

	dispatchEvent(event: RivetEvent): boolean {
		this.#dispatch(event.type, this.#createEvent(event.type, event));
		return true;
	}

	get onopen(): ((event: RivetEvent) => void | Promise<void>) | null {
		return this.#onopen;
	}

	set onopen(fn: ((event: RivetEvent) => void | Promise<void>) | null) {
		this.#onopen = fn;
	}

	get onclose(): ((event: RivetCloseEvent) => void | Promise<void>) | null {
		return this.#onclose;
	}

	set onclose(fn: ((event: RivetCloseEvent) => void | Promise<void>) | null) {
		this.#onclose = fn;
	}

	get onerror(): ((event: RivetEvent) => void | Promise<void>) | null {
		return this.#onerror;
	}

	set onerror(fn: ((event: RivetEvent) => void | Promise<void>) | null) {
		this.#onerror = fn;
	}

	get onmessage():
		| ((event: RivetMessageEvent) => void | Promise<void>)
		| null {
		return this.#onmessage;
	}

	set onmessage(fn:
		| ((event: RivetMessageEvent) => void | Promise<void>)
		| null,) {
		this.#onmessage = fn;
	}

	#createEvent(type: string, event: any): any {
		switch (type) {
			case "message":
				return {
					type,
					data: event.data,
					rivetMessageIndex: event.rivetMessageIndex,
					target: this,
					currentTarget: this,
				} satisfies RivetMessageEvent;
			case "close":
				return {
					type,
					code: event.code,
					reason: event.reason,
					wasClean: event.wasClean,
					target: this,
					currentTarget: this,
				} satisfies RivetCloseEvent;
			default:
				return {
					type,
					target: this,
					currentTarget: this,
					...(event.message !== undefined
						? { message: event.message }
						: {}),
					...(event.error !== undefined
						? { error: event.error }
						: {}),
				} satisfies RivetEvent;
		}
	}

	#dispatch(type: string, event: any): void {
		const listeners = this.#listeners.get(type);
		if (listeners && listeners.length > 0) {
			for (const listener of [...listeners]) {
				this.#callHandler(type, listener, event);
			}
		}

		switch (type) {
			case "open":
				if (this.#onopen) this.#callHandler(type, this.#onopen, event);
				break;
			case "close":
				if (this.#onclose)
					this.#callHandler(type, this.#onclose, event);
				break;
			case "error":
				if (this.#onerror)
					this.#callHandler(type, this.#onerror, event);
				break;
			case "message":
				if (this.#onmessage)
					this.#callHandler(type, this.#onmessage, event);
				break;
		}
	}

	#callHandler(
		type: string,
		handler: TrackedWebSocketListener,
		event: any,
	): void {
		try {
			const result = handler(event);
			if (!this.#isPromiseLike(result)) {
				return;
			}
			const callbackRegionId = this.#ctx.beginWebSocketCallback();
			this.#ctx.waitUntil(
				Promise.resolve(result)
					.catch((error) => {
						logger().error({
							msg: "async websocket handler failed",
							eventType: type,
							error,
						});
					})
					.finally(() => {
						this.#ctx.endWebSocketCallback(callbackRegionId);
					}),
			);
		} catch (error) {
			logger().error({
				msg: "websocket handler failed",
				eventType: type,
				error,
			});
		}
	}

	#isPromiseLike(value: unknown): value is PromiseLike<void> {
		return (
			typeof value === "object" &&
			value !== null &&
			"then" in value &&
			typeof value.then === "function"
		);
	}
}

export class NativeActorContextAdapter {
	#bindings: NativeBindings;
	#ctx: NativeActorContext;
	#schemas: NativeValidationConfig;
	#abortSignal?: AbortSignal;
	#abortSignalCleanup?: () => void;
	#client?: AnyClient;
	#clientFactory?: () => AnyClient;
	#databaseProvider?: Exclude<AnyDatabaseProvider, undefined>;
	#db?: unknown;
	#dbProxy?: unknown;
	#dispatchCancelToken?: NativeCancellationToken;
	#kv?: NativeKvAdapter;
	#queue?: NativeQueueAdapter;
	#request?: Request;
	#schedule?: NativeScheduleAdapter;
	#sql?: ReturnType<typeof wrapJsNativeDatabase>;
	#runHandlerActiveProvider?: () => boolean;
	#onStateChange?: NativeOnStateChangeHandler;
	#stateEnabled: boolean;

	constructor(
		bindings: NativeBindings,
		ctx: NativeActorContext,
		clientFactory?: () => AnyClient,
		schemas: NativeValidationConfig = {},
		databaseProvider?: AnyDatabaseProvider,
		request?: Request,
		stateEnabled = true,
		runHandlerActiveProvider?: () => boolean,
		onStateChange?: NativeOnStateChangeHandler,
		dispatchCancelToken?: NativeCancellationToken,
	) {
		this.#bindings = bindings;
		this.#ctx = ctx;
		this.#clientFactory = clientFactory;
		this.#schemas = schemas;
		this.#dispatchCancelToken = dispatchCancelToken;
		this.#runHandlerActiveProvider = runHandlerActiveProvider;
		this.#onStateChange = onStateChange;
		this.#stateEnabled = stateEnabled;
		if (databaseProvider) {
			this.#databaseProvider = databaseProvider;
		}
		this.#request = request;
		(
			this as NativeActorContextAdapter & {
				[ACTOR_CONTEXT_INTERNAL_SYMBOL]?: unknown;
			}
		)[ACTOR_CONTEXT_INTERNAL_SYMBOL] = new NativeWorkflowRuntimeAdapter(
			this,
		);
	}

	get kv() {
		if (!this.#kv) {
			this.#kv = new NativeKvAdapter(this.#ctx.kv());
		}
		return this.#kv;
	}

	get sql() {
		if (!this.#sql) {
			this.#sql = getOrCreateNativeSqlDatabase(this.#ctx);
		}
		return this.#sql;
	}

	get db() {
		if (!this.#databaseProvider) {
			throw databaseNotConfiguredError();
		}

		if (!this.#dbProxy) {
			this.#dbProxy = new Proxy(
				{},
				{
					get: (_target, property) => {
						if (property === "then") {
							return undefined;
						}

						return async (...args: Array<unknown>) => {
							const client = await this.ensureDatabaseClient();
							const value = Reflect.get(
								client as object,
								property,
							);
							if (typeof value !== "function") {
								return value;
							}
							return await value.apply(client, args);
						};
					},
				},
			);
		}

		return this.#dbProxy;
	}

	get state(): unknown {
		if (!this.#stateEnabled) {
			throw stateNotEnabledError();
		}
		const nextState = this.#readState();
		return createWriteThroughProxy(
			nextState,
			(nextValue) => {
				this.#writeState(nextValue, { scheduleSave: true });
			},
			() => {
				this.#assertCanMutateState();
			},
		);
	}

	set state(value: unknown) {
		if (!this.#stateEnabled) {
			throw stateNotEnabledError();
		}
		this.#assertCanMutateState();
		this.#writeState(value, { scheduleSave: true });
	}

	initializeState(value: unknown): void {
		if (!this.#stateEnabled) {
			return;
		}
		this.#writeState(value, { scheduleSave: false });
	}

	get vars(): unknown {
		const runtimeState = getNativeRuntimeState(this.#ctx);
		if (runtimeState.varsInitialized) {
			return runtimeState.vars;
		}

		runtimeState.varsInitialized = true;
		runtimeState.vars = undefined;
		return undefined;
	}

	set vars(value: unknown) {
		const runtimeState = getNativeRuntimeState(this.#ctx);
		runtimeState.varsInitialized = true;
		runtimeState.vars = value;
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

	get key(): string[] {
		return toActorKey(callNativeSync(() => this.#ctx.key()));
	}

	get region(): string {
		return callNativeSync(() => this.#ctx.region());
	}

	get conns(): Map<string, NativeConnAdapter> {
		return new Map(
			callNativeSync(() => this.#ctx.conns()).map((conn) => [
				conn.id(),
				new NativeConnAdapter(
					conn,
					this.#schemas,
					this.#ctx,
					(connId) =>
						callNativeSync(() =>
							this.#ctx.queueHibernationRemoval(connId),
						),
				),
			]),
		);
	}

	get log() {
		return logger();
	}

	get abortSignal(): AbortSignal {
		if (!this.#abortSignal) {
			const actorSignal = this.#createActorAbortSignal();
			if (this.#dispatchCancelToken === undefined) {
				this.#abortSignal = actorSignal;
			} else {
				const controller = new AbortController();
				let cleanedUp = false;
				const onActorAbort = () => {
					cleanup();
					controller.abort();
				};
				const cleanup = () => {
					if (cleanedUp) {
						return;
					}
					cleanedUp = true;
					actorSignal.removeEventListener("abort", onActorAbort);
					this.#abortSignalCleanup = undefined;
				};

				if (
					actorSignal.aborted ||
					this.#dispatchCancelToken.aborted()
				) {
					controller.abort();
				} else {
					const dispatchCancelToken = this.#dispatchCancelToken;
					this.#abortSignalCleanup = cleanup;
					actorSignal.addEventListener("abort", onActorAbort, {
						once: true,
					});
					callNativeSync(() =>
						dispatchCancelToken.onCancelled(() => {
							cleanup();
							controller.abort();
						}),
					);
				}

				this.#abortSignal = controller.signal;
			}
		}
		return this.#abortSignal;
	}

	get aborted(): boolean {
		return this.abortSignal.aborted;
	}

	get request(): Request | undefined {
		return this.#request;
	}

	private async ensureDatabaseClient(): Promise<unknown> {
		if (!this.#databaseProvider) {
			throw databaseNotConfiguredError();
		}

		if (this.#db) {
			return this.#db;
		}

		const runtimeState = getNativeRuntimeState(this.#ctx);
		const cachedClient = runtimeState.databaseClient;
		if (cachedClient) {
			this.#db = cachedClient.client;
			return this.#db;
		}

		const actorId = this.actorId;
		const client = await this.#databaseProvider.createClient({
			actorId,
			kv: {
				batchPut: async (entries) => {
					await this.kv.batchPut(
						entries.map(([key, value]) => [key, value]),
					);
				},
				batchGet: async (keys) => {
					return await this.kv.batchGet([...keys]);
				},
				batchDelete: async (keys) => {
					await this.kv.batchDelete([...keys]);
				},
				deleteRange: async (start, end) => {
					await this.kv.deleteRange(start, end);
				},
			},
			log: {
				debug: (obj) => logger().debug(obj),
			},
			nativeDatabaseProvider: {
				open: async (requestedActorId) => {
					void requestedActorId;
					return getOrCreateNativeSqlDatabase(this.#ctx);
				},
			},
		});
		runtimeState.databaseClient = {
			client,
			provider: this.#databaseProvider,
		};
		this.#db = client;
		return client;
	}

	async prepare(): Promise<void> {
		if (!this.#databaseProvider) {
			return;
		}

		await this.ensureDatabaseClient();
	}

	async runDatabaseMigrations(): Promise<void> {
		if (!this.#databaseProvider) {
			return;
		}

		await this.#databaseProvider.onMigrate(
			(await this.ensureDatabaseClient()) as never,
		);
	}

	async closeDatabase(destroy: boolean): Promise<void> {
		this.#db = undefined;
		this.#sql = undefined;
		await closeNativeDatabaseClient(this.#ctx, destroy);
		await closeNativeSqlDatabase(this.#ctx);
	}

	broadcast(name: string, ...args: unknown[]): void {
		const validatedArgs = validateEventArgs(
			this.#schemas.events,
			name,
			args,
		);
		callNativeSync(() =>
			this.#ctx.broadcast(name, encodeValue(validatedArgs)),
		);
	}

	async saveState(opts?: {
		immediate?: boolean;
		maxWait?: number;
	}): Promise<void> {
		if (opts?.immediate) {
			await callNative(() =>
				this.#ctx.requestSaveAndWait({ immediate: true }),
			);
			return;
		}

		if (opts?.maxWait != null) {
			callNativeSync(() =>
				this.#ctx.requestSave({ maxWaitMs: opts.maxWait }),
			);
			return;
		}

		callNativeSync(() => this.#ctx.requestSave({ immediate: false }));
	}

	serializeForTick(reason: SerializeStateReason): NativeStateDeltaPayload {
		void reason;
		const actorState = getNativePersistState(this.#ctx);
		const connHibernationRemoved = callNativeSync(() =>
			this.#ctx.takePendingHibernationChanges(),
		);
		for (const connId of connHibernationRemoved) {
			actorState.connStates.delete(connId);
		}
		const state =
			this.#stateEnabled && this.#readState() !== undefined
				? Buffer.from(encodeValue(this.#readState()))
				: undefined;
		const connHibernation = callNativeSync(() =>
			this.#ctx.dirtyHibernatableConns(),
		).map((conn) => {
			const connId = callNativeSync(() => conn.id());
			return {
				connId,
				bytes: Buffer.from(callNativeSync(() => conn.state())),
			};
		});

		return {
			state,
			connHibernation,
			connHibernationRemoved,
		};
	}

	async restartRunHandler(): Promise<void> {
		await callNative(async () => {
			this.#ctx.restartRunHandler();
		});
	}

	async setAlarm(timestampMs?: number): Promise<void> {
		await callNative(async () => {
			this.#ctx.setAlarm(timestampMs);
		});
	}

	keepAwake<T>(promise: Promise<T>): Promise<T> {
		const trackedPromise = promise.then(() => null);
		try {
			callNativeSync(() => this.#ctx.registerTask(trackedPromise));
		} catch (error) {
			if (!isClosedTaskRegistrationError(error)) {
				throw error;
			}
		}
		return promise;
	}

	runHandlerActive(): boolean {
		return this.#runHandlerActiveProvider?.() ?? false;
	}

	internalKeepAwake<T>(
		run: Promise<T> | (() => Promise<T>),
	): Promise<T> {
		const promise = typeof run === "function" ? run() : run;
		const trackedPromise = promise.then(() => null);
		try {
			callNativeSync(() => this.#ctx.registerTask(trackedPromise));
		} catch (error) {
			if (!isClosedTaskRegistrationError(error)) {
				throw error;
			}
		}
		return promise;
	}

	waitUntil(promise: Promise<unknown>): void {
		void callNative(() => this.#ctx.waitUntil(Promise.resolve(promise)));
	}

	beginWebSocketCallback(): number {
		return callNativeSync(() => this.#ctx.beginWebsocketCallback());
	}

	endWebSocketCallback(callbackRegionId: number): void {
		callNativeSync(() =>
			this.#ctx.endWebsocketCallback(callbackRegionId),
		);
	}

	setPreventSleep(preventSleep: boolean): void {
		callNativeSync(() => this.#ctx.setPreventSleep(preventSleep));
	}

	get preventSleep(): boolean {
		return callNativeSync(() => this.#ctx.preventSleep());
	}

	sleep(): void {
		callNativeSync(() => this.#ctx.sleep());
	}

	destroy(): void {
		markNativeDestroyRequested(this.#ctx);
		callNativeSync(() => this.#ctx.destroy());
	}

	client<T = AnyClient>(): T extends Registry<any> ? Client<T> : T {
		if (!this.#client) {
			if (!this.#clientFactory) {
				throw nativeClientNotConfiguredError();
			}
			this.#client = this.#clientFactory();
		}

		return this.#client as T extends Registry<any> ? Client<T> : T;
	}

	async dispose(): Promise<void> {
		this.#abortSignalCleanup?.();
		this.#sql = undefined;
	}

	#createActorAbortSignal(): AbortSignal {
		const nativeSignal = callNativeSync(() => this.#ctx.abortSignal());
		const controller = new AbortController();
		if (nativeSignal.aborted) {
			controller.abort();
		} else {
			nativeSignal.addEventListener(
				"abort",
				() => controller.abort(),
				{ once: true },
			);
		}
		return controller.signal;
	}

	#readState(): unknown {
		const actorState = getNativePersistState(this.#ctx);
		if (actorState.state === undefined) {
			actorState.state = decodeValue(callNativeSync(() => this.#ctx.state()));
		}
		return actorState.state;
	}

	#writeState(
		value: unknown,
		options: {
			scheduleSave: boolean;
		},
	): void {
		encodeValue(value);
		const actorState = getNativePersistState(this.#ctx);
		actorState.state = value;
		if (!options.scheduleSave) {
			return;
		}
		this.#handleStateChange();
	}

	#assertCanMutateState(): void {
		const actorState = getNativePersistState(this.#ctx);
		if (actorState.isInOnStateChange) {
			throw stateMutationReentrantError();
		}
	}

	#handleStateChange(): void {
		const actorState = getNativePersistState(this.#ctx);
		encodeValue(actorState.state);
		callNativeSync(() => this.#ctx.requestSave({ immediate: false }));

		if (!this.#onStateChange) {
			return;
		}

		actorState.isInOnStateChange = true;
		callNativeSync(() => this.#ctx.beginOnStateChange());
		let shouldFinish = true;
		try {
			const result = this.#onStateChange(this, actorState.state) as unknown;
			if (isPromiseLike(result)) {
				shouldFinish = false;
				void Promise.resolve(result)
					.catch((error) => {
						logger().error({
							msg: "error in `onStateChange`",
							error,
						});
					})
					.finally(() => {
						actorState.isInOnStateChange = false;
						callNativeSync(() => this.#ctx.endOnStateChange());
					});
			}
		} finally {
			if (shouldFinish) {
				actorState.isInOnStateChange = false;
				callNativeSync(() => this.#ctx.endOnStateChange());
			}
		}
	}
}

type NativeWorkflowQueueMessage = Awaited<
	ReturnType<NativeQueueAdapter["next"]>
>;

class NativeWorkflowRuntimeAdapter {
	#ctx: NativeActorContextAdapter;
	#completions = new Map<string, (response?: unknown) => Promise<void>>();

	readonly id: string;
	readonly driver: {
		kvBatchGet: (
			actorId: string,
			keys: Uint8Array[],
		) => Promise<Array<Uint8Array | null>>;
		kvBatchPut: (
			actorId: string,
			entries: Array<[Uint8Array, Uint8Array]>,
		) => Promise<void>;
		kvBatchDelete: (actorId: string, keys: Uint8Array[]) => Promise<void>;
		kvDeleteRange: (
			actorId: string,
			start: Uint8Array,
			end: Uint8Array,
		) => Promise<void>;
		kvListPrefix: (
			actorId: string,
			prefix: Uint8Array,
		) => Promise<Array<[Uint8Array, Uint8Array]>>;
		setAlarm: (_actor: unknown, wakeAt: number) => Promise<void>;
	};
	readonly queueManager: {
		enqueue: (name: string, body: unknown) => Promise<unknown>;
		receive: (
			names: string[] | undefined,
			count: number,
			timeout?: number,
			_abortSignal?: AbortSignal,
			completable?: boolean,
		) => Promise<
			Array<{
				id: bigint;
				name: string;
				body: unknown;
				createdAt: number;
				complete?: (response?: unknown) => Promise<void>;
			}>
		>;
		completeMessage: (
			message: {
				id: bigint;
				complete?: (response?: unknown) => Promise<void>;
			},
			response?: unknown,
		) => Promise<void>;
		completeMessageById: (
			messageId: bigint,
			response?: unknown,
		) => Promise<void>;
		waitForNames: (
			names: readonly string[] | undefined,
			abortSignal?: AbortSignal,
		) => Promise<void>;
	};
	readonly stateManager: {
		saveState: (opts?: {
			immediate?: boolean;
			maxWait?: number;
		}) => Promise<void>;
	};

	constructor(ctx: NativeActorContextAdapter) {
		this.#ctx = ctx;
		this.id = ctx.actorId;
		this.driver = {
			kvBatchGet: async (actorId, keys) => {
				this.#assertActorId(actorId);
				return await this.#ctx.kv.batchGet(keys);
			},
			kvBatchPut: async (actorId, entries) => {
				this.#assertActorId(actorId);
				await this.#ctx.kv.batchPut(entries);
			},
			kvBatchDelete: async (actorId, keys) => {
				this.#assertActorId(actorId);
				await this.#ctx.kv.batchDelete(keys);
			},
			kvDeleteRange: async (actorId, start, end) => {
				this.#assertActorId(actorId);
				await this.#ctx.kv.rawDeleteRange(start, end);
			},
			kvListPrefix: async (actorId, prefix) => {
				this.#assertActorId(actorId);
				return await this.#ctx.kv.rawListPrefix(prefix);
			},
			setAlarm: async (_actor, wakeAt) => {
				await this.#ctx.setAlarm(wakeAt);
			},
		};
		this.queueManager = {
			enqueue: async (name, body) => {
				return this.#wrapQueueMessage(
					await this.#ctx.queue.send(name, body),
				);
			},
			receive: async (
				names,
				count,
				timeout,
				_abortSignal,
				completable,
			) => {
				const messages = await this.#ctx.queue.nextBatch({
					names,
					count,
					timeout: timeout ?? 0,
					completable,
				});
				return messages.map((message) =>
					this.#wrapQueueMessage(message),
				);
			},
			completeMessage: async (message, response) => {
				await message.complete?.(response);
				this.#completions.delete(message.id.toString());
			},
			completeMessageById: async (messageId, response) => {
				const complete = this.#completions.get(messageId.toString());
				if (!complete) {
					return;
				}
				await complete(response);
				this.#completions.delete(messageId.toString());
			},
			waitForNames: async (names, abortSignal) => {
				await this.#ctx.queue.waitForNamesAvailable(names ?? [], {
					signal: abortSignal,
				});
			},
		};
		this.stateManager = {
			saveState: async (opts) => {
				await this.#ctx.saveState(opts);
			},
		};
	}

	isRunHandlerActive(): boolean {
		return this.#ctx.runHandlerActive();
	}

	async restartRunHandler(): Promise<void> {
		await this.#ctx.restartRunHandler();
	}

	#assertActorId(actorId: string): void {
		if (actorId !== this.id) {
			throw new Error(
				`workflow runtime actor id mismatch: expected ${this.id}, got ${actorId}`,
			);
		}
	}

	#wrapQueueMessage(message: NativeWorkflowQueueMessage) {
		if (!message) {
			throw new Error("native workflow queue message missing");
		}

		const id = BigInt(message.id);
		let complete: ((response?: unknown) => Promise<void>) | undefined;
		if (message.complete) {
			complete = async (response?: unknown) => {
				await message.complete?.(response);
			};
			this.#completions.set(id.toString(), complete);
		}

		return {
			id,
			name: message.name,
			body: message.body,
			createdAt: message.createdAt,
			complete,
		};
	}
}

function buildNativeHttpRequest(
	request: Request,
	body?: Uint8Array,
): {
	method: string;
	uri: string;
	headers: Record<string, string>;
	body?: Buffer;
} {
	return {
		method: request.method,
		uri: request.url,
		headers: Object.fromEntries(request.headers.entries()),
		body: body && body.byteLength > 0 ? Buffer.from(body) : undefined,
	};
}

function withConnContext(
	bindings: NativeBindings,
	ctx: NativeActorContext,
	conn: NativeConnHandle,
	clientFactory?: () => AnyClient,
	schemas: NativeValidationConfig = {},
	databaseProvider?: AnyDatabaseProvider,
	request?: Request,
	stateEnabled = true,
	onStateChange?: NativeOnStateChangeHandler,
	dispatchCancelToken?: NativeCancellationToken,
) {
	return Object.assign(
		new NativeActorContextAdapter(
			bindings,
			ctx,
			clientFactory,
			schemas,
			databaseProvider,
			request,
			stateEnabled,
			undefined,
			onStateChange,
			dispatchCancelToken,
		),
		{
			conn: new NativeConnAdapter(
				conn,
				schemas,
				ctx,
				(connId) =>
					callNativeSync(() =>
						ctx.queueHibernationRemoval(connId),
					),
			),
		},
	);
}

function buildNativeRequestErrorResponse(
	encoding: Encoding,
	path: string,
	error: unknown,
): Response {
	const { statusCode, group, code, message, metadata } = deconstructError(
		error,
		logger(),
		{
			path,
			runtime: "native",
		},
		false,
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
					: bufferToArrayBuffer(encodeCborCompat(value.metadata)),
		}),
	);

	return new Response(body, {
		status: statusCode,
		headers: {
			"Content-Type": contentTypeForEncoding(encoding),
		},
	});
}

function buildActorConfig(
	definition: AnyActorDefinition,
	registryConfig: RegistryConfig,
): JsActorConfig {
	const config = definition.config as unknown as Record<string, unknown>;
	const options = (config.options ?? {}) as Record<string, unknown>;
	const canHibernate = options.canHibernateWebSocket;

	return {
		name: options.name as string | undefined,
		icon: options.icon as string | undefined,
		hasDatabase: config.db !== undefined,
		hasState:
			config.state !== undefined ||
			typeof config.createState === "function",
		canHibernateWebsocket:
			typeof canHibernate === "boolean" ? canHibernate : undefined,
		stateSaveIntervalMs: options.stateSaveInterval as number | undefined,
		createVarsTimeoutMs: options.createVarsTimeout as number | undefined,
		createConnStateTimeoutMs: options.createConnStateTimeout as
			| number
			| undefined,
		onBeforeConnectTimeoutMs: options.onBeforeConnectTimeout as
			| number
			| undefined,
		onConnectTimeoutMs: options.onConnectTimeout as number | undefined,
		onMigrateTimeoutMs: options.onMigrateTimeout as number | undefined,
		onDestroyTimeoutMs: options.onDestroyTimeout as number | undefined,
		actionTimeoutMs: options.actionTimeout as number | undefined,
		sleepTimeoutMs: options.sleepTimeout as number | undefined,
		noSleep: options.noSleep as boolean | undefined,
		sleepGracePeriodMs: options.sleepGracePeriod as number | undefined,
		connectionLivenessTimeoutMs: options.connectionLivenessTimeout as
			| number
			| undefined,
		connectionLivenessIntervalMs: options.connectionLivenessInterval as
			| number
			| undefined,
		maxQueueSize: options.maxQueueSize as number | undefined,
		maxQueueMessageSize: options.maxQueueMessageSize as number | undefined,
		maxIncomingMessageSize: registryConfig.maxIncomingMessageSize as
			| number
			| undefined,
		maxOutgoingMessageSize: registryConfig.maxOutgoingMessageSize as
			| number
			| undefined,
		preloadMaxWorkflowBytes: options.preloadMaxWorkflowBytes as
			| number
			| undefined,
		preloadMaxConnectionsBytes: options.preloadMaxConnectionsBytes as
			| number
			| undefined,
		actions: Object.keys(
			(config.actions ?? {}) as Record<string, unknown>,
		)
			.sort()
			.map((name) => ({ name })),
	};
}

export function buildNativeFactory(
	bindings: NativeBindings,
	registryConfig: RegistryConfig,
	definition: AnyActorDefinition,
): NativeActorFactory {
	const config = definition.config as Record<string, any>;
	const databaseProvider = config.db as AnyDatabaseProvider;
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
	const nativeRunHandlerActiveByActorId = new Map<string, boolean>();
	const isNativeRunHandlerActive = (ctx: NativeActorContext) =>
		nativeRunHandlerActiveByActorId.get(
			callNativeSync(() => ctx.actorId()),
		) ?? false;
	const getNativeWorkflowInspector = (ctx: NativeActorContext) =>
		getRunInspectorConfig(
			config.run,
			callNativeSync(() => ctx.actorId()),
		)?.workflow as NativeWorkflowInspectorConfig | undefined;
	const onStateChange =
		typeof config.onStateChange === "function"
			? (actorCtx: NativeActorContextAdapter, nextState: unknown) => {
					config.onStateChange(actorCtx, nextState);
				}
			: undefined;
	const hasStaticState = "state" in config;
	const hasStaticVars = "vars" in config;
	const hasStaticConnState = Object.hasOwn(config, "connState");
	const hasDynamicConnState = typeof config.createConnState === "function";
	const needsDisconnectCallback =
		typeof config.onDisconnect === "function" ||
		hasStaticConnState ||
		hasDynamicConnState ||
		config.options?.canHibernateWebSocket === true;
	const stateEnabled =
		config.state !== undefined || typeof config.createState === "function";
	const makeActorCtx = (
		ctx: NativeActorContext,
		request?: Request,
		cancelToken?: NativeCancellationToken,
	) =>
		new NativeActorContextAdapter(
			bindings,
			ctx,
			createClient,
			schemaConfig,
			databaseProvider,
			request,
			stateEnabled,
			() => isNativeRunHandlerActive(ctx),
			onStateChange,
			cancelToken,
		);
	const makeConnCtx = (
		ctx: NativeActorContext,
		conn: NativeConnHandle,
		request?: Request,
		cancelToken?: NativeCancellationToken,
	) =>
		withConnContext(
			bindings,
			ctx,
			conn,
			createClient,
			schemaConfig,
			databaseProvider,
			request,
			stateEnabled,
			onStateChange,
			cancelToken,
		);
	const maybeHandleNativeInspectorRequest = async (
		ctx: NativeActorContext,
		rawRequest: {
			method: string;
			uri: string;
			headers?: Record<string, string>;
			body?: Buffer;
		},
		jsRequest: Request,
	): Promise<Response | undefined> => {
		const url = new URL(jsRequest.url);
		if (!url.pathname.startsWith("/inspector/")) {
			return undefined;
		}

		const jsonResponse = (body: unknown, init?: ResponseInit) =>
			new Response(JSON.stringify(body), {
				status: init?.status ?? 200,
				headers: {
					"Content-Type": "application/json",
					...(init?.headers ?? {}),
				},
			});
		const errorResponse = (error: unknown, status?: number) => {
			const rivetError = toRivetError(error);
			return jsonResponse(
				{
					group: rivetError.group,
					code: rivetError.code,
					message: rivetError.message,
					metadata: rivetError.metadata ?? null,
				},
				{
					status:
						status ??
						rivetError.statusCode ??
						(rivetError.public ? 400 : 500),
				},
			);
		};
		try {
			await ctx.verifyInspectorAuth(
				jsRequest.headers.get("authorization")?.replace(/^Bearer\s+/i, "") ??
					null,
			);
		} catch (error) {
			return errorResponse(error, 401);
		}

		const workflowHistory = () =>
			serializeWorkflowHistoryForJson(
				getNativeWorkflowInspector(ctx)?.getHistory() ?? null,
			);
		const workflowState = async () =>
			(await getNativeWorkflowInspector(ctx)?.getState?.()) ?? null;
		const actorCtx = makeActorCtx(ctx, jsRequest);
		try {
			if (
				url.pathname === "/inspector/state" &&
				jsRequest.method === "GET"
			) {
				return jsonResponse({
					state: stateEnabled ? actorCtx.state : undefined,
					isStateEnabled: stateEnabled,
				});
			}
			if (
				url.pathname === "/inspector/state" &&
				jsRequest.method === "PATCH"
			) {
				const body = (await jsRequest.json()) as { state?: unknown };
				actorCtx.state = body.state;
				await actorCtx.saveState({ immediate: true });
				return jsonResponse({ ok: true });
			}
			if (
				url.pathname === "/inspector/connections" &&
				jsRequest.method === "GET"
			) {
				return jsonResponse({
					connections: Array.from(actorCtx.conns.values()).map(
						(conn) => ({
							type: null,
							id: conn.id,
							details: {
								type: null,
								params: conn.params,
								stateEnabled: true,
								state: conn.state,
								subscriptions: 0,
								isHibernatable: conn.isHibernatable,
							},
						}),
					),
				});
			}
			if (
				url.pathname === "/inspector/rpcs" &&
				jsRequest.method === "GET"
			) {
				return jsonResponse({
					rpcs: Object.keys(actionHandlers).sort(),
				});
			}
			if (
				url.pathname === "/inspector/queue" &&
				jsRequest.method === "GET"
			) {
				const limitParam = url.searchParams.get("limit");
				const parsedLimit = limitParam ? Number(limitParam) : 100;
				const limit =
					Number.isFinite(parsedLimit) && parsedLimit > 0
						? Math.floor(parsedLimit)
						: 100;
				const queue = ctx.queue();
				const allMessages = await queue.inspectMessages();
				const truncated = allMessages.length > limit;
				const messages = allMessages
					.slice(0, limit)
					.map((m) => ({
						id: m.id,
						name: m.name,
						createdAtMs: m.createdAtMs,
					}));
				return jsonResponse({
					size: allMessages.length,
					maxSize: queue.maxSize(),
					truncated,
					messages,
				});
			}
			if (
				url.pathname === "/inspector/traces" &&
				jsRequest.method === "GET"
			) {
				return jsonResponse({ otlp: [], clamped: false });
			}
			if (
				url.pathname === "/inspector/workflow-history" &&
				jsRequest.method === "GET"
			) {
				return jsonResponse({
					history: workflowHistory(),
					workflowState: await workflowState(),
					isWorkflowEnabled:
						getNativeWorkflowInspector(ctx) !== undefined,
				});
			}
			if (
				url.pathname === "/inspector/workflow/replay" &&
				jsRequest.method === "POST"
			) {
				try {
					const body = (await jsRequest.json()) as {
						entryId?: string;
					};
					const history = await getNativeWorkflowInspector(
						ctx,
					)?.replayFromStep?.(body.entryId);
					return jsonResponse({
						history: serializeWorkflowHistoryForJson(
							history ?? null,
						),
						workflowState: await workflowState(),
						isWorkflowEnabled:
							getNativeWorkflowInspector(ctx) !== undefined,
					});
				} catch (error) {
					return errorResponse(error);
				}
			}
			if (
				url.pathname === "/inspector/database/schema" &&
				jsRequest.method === "GET"
			) {
				const db = actorCtx.sql;
				const tables = queryRows(
					await db.query(
						"SELECT name, type FROM sqlite_master WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '__drizzle_%' ORDER BY name",
					),
				) as Array<{ name: string; type: string }>;
				const tableInfos = [];
				for (const table of tables) {
					const quoted = `"${table.name.replace(/"/g, '""')}"`;
					const columns = queryRows(
						await db.query(`PRAGMA table_info(${quoted})`),
					);
					const foreignKeys = queryRows(
						await db.query(`PRAGMA foreign_key_list(${quoted})`),
					);
					const countResult = queryRows(
						await db.query(
							`SELECT COUNT(*) as count FROM ${quoted}`,
						),
					) as Array<{ count?: number }>;
					tableInfos.push({
						table: {
							schema: "main",
							name: table.name,
							type: table.type,
						},
						columns: jsonSafe(columns),
						foreignKeys: jsonSafe(foreignKeys),
						records: countResult[0]?.count ?? 0,
					});
				}
				return jsonResponse({ schema: { tables: tableInfos } });
			}
			if (
				url.pathname === "/inspector/database/rows" &&
				jsRequest.method === "GET"
			) {
				const table = url.searchParams.get("table");
				if (!table) {
					return jsonResponse(
						{ error: "Missing required table query parameter" },
						{ status: 400 },
					);
				}
				const limit = Number.parseInt(
					url.searchParams.get("limit") ?? "100",
					10,
				);
				const offset = Number.parseInt(
					url.searchParams.get("offset") ?? "0",
					10,
				);
				const quoted = `"${table.replace(/"/g, '""')}"`;
				const rows = queryRows(
					await actorCtx.sql.query(
						`SELECT * FROM ${quoted} LIMIT ? OFFSET ?`,
						[
							Math.max(0, Math.min(limit, 500)),
							Math.max(0, offset),
						],
					),
				);
				return jsonResponse({ rows: jsonSafe(rows) });
			}
			if (
				url.pathname === "/inspector/database/execute" &&
				jsRequest.method === "POST"
			) {
				const body = (await jsRequest.json()) as {
					sql?: unknown;
					args?: unknown;
					properties?: unknown;
				};
				if (typeof body.sql !== "string" || body.sql.trim() === "") {
					return jsonResponse(
						{ error: "sql is required" },
						{ status: 400 },
					);
				}
				if (
					Array.isArray(body.args) &&
					body.properties &&
					typeof body.properties === "object"
				) {
					return jsonResponse(
						{ error: "use either args or properties, not both" },
						{ status: 400 },
					);
				}
				const readOnly = /^\s*(SELECT|PRAGMA|WITH|EXPLAIN)\b/i.test(
					body.sql,
				);
				if (
					body.properties &&
					typeof body.properties === "object" &&
					!Array.isArray(body.properties)
				) {
					const bindings = normalizeSqlitePropertyBindings(
						body.properties as Record<string, unknown>,
					);
					if (readOnly) {
						const rows = queryRows(
							await actorCtx.sql.query(body.sql, bindings),
						);
						return jsonResponse({ rows: jsonSafe(rows) });
					}
					await actorCtx.sql.run(body.sql, bindings);
					return jsonResponse({ rows: [] });
				}
				const args = Array.isArray(body.args) ? body.args : [];
				if (readOnly) {
					const rows = queryRows(
						await actorCtx.sql.query(body.sql, args),
					);
					return jsonResponse({ rows: jsonSafe(rows) });
				}
				await actorCtx.sql.run(body.sql, args);
				return jsonResponse({ rows: [] });
			}
			if (
				url.pathname === "/inspector/summary" &&
				jsRequest.method === "GET"
			) {
				const inspectorSnapshot = callNativeSync(() =>
					ctx.inspectorSnapshot(),
				);
				return jsonResponse({
					state: stateEnabled ? actorCtx.state : undefined,
					connections: Array.from(actorCtx.conns.values()).map(
						(conn) => ({
							type: null,
							id: conn.id,
							details: {
								type: null,
								params: conn.params,
								stateEnabled: true,
								state: conn.state,
								subscriptions: 0,
								isHibernatable: conn.isHibernatable,
							},
						}),
					),
					rpcs: Object.keys(actionHandlers).sort(),
					queueSize: inspectorSnapshot.queueSize,
					isStateEnabled: stateEnabled,
					isDatabaseEnabled: databaseProvider !== undefined,
					isWorkflowEnabled:
						getNativeWorkflowInspector(ctx) !== undefined,
					workflowState: await workflowState(),
					workflowHistory: workflowHistory(),
				});
			}
			if (
				jsRequest.method === "POST" &&
				url.pathname.startsWith("/inspector/action/")
			) {
				const actionName = url.pathname.replace(
					"/inspector/action/",
					"",
				);
				const action = actionHandlers[actionName];
				if (!action) {
					return errorResponse(
						new RivetError(
							"action",
							"action_not_found",
							`Action ${actionName} not found`,
						),
						404,
					);
				}
				const body = (await jsRequest.json()) as { args?: unknown[] };
				try {
					const output = await action(
						actorCtx,
						...validateActionArgs(
							schemaConfig.actionInputSchemas,
							actionName,
							body.args ?? [],
						),
					);
					return jsonResponse({ output });
				} catch (error) {
					return errorResponse(error);
				}
			}

			return jsonResponse(
				{
					group: "actor",
					code: "not_found",
					message: "Inspector route was not found",
					metadata: null,
				},
				{ status: 404 },
			);
		} catch (error) {
			return errorResponse(error);
		} finally {
			await actorCtx.dispose();
		}
		};
	const callbacks = {
		createState:
			hasStaticState || typeof config.createState === "function"
				? wrapNativeCallback(
						async (
							error: unknown,
							payload: {
								ctx: NativeActorContext;
								input?: Buffer;
							},
						): Promise<Buffer> => {
							const { ctx, input } = unwrapTsfnPayload(error, payload);
							const actorCtx = makeActorCtx(ctx);
							try {
								const decodedInput = decodeValue(input);
								const startedAt = performance.now();
								const state = hasStaticState
									? structuredClone(config.state)
									: await config.createState(actorCtx, decodedInput);
								logger().debug({
									msg: "perf user: createStateMs",
									durationMs: performance.now() - startedAt,
								});
								actorCtx.initializeState(state);
								return encodeValue(state);
							} finally {
								await actorCtx.dispose();
							}
						},
					)
				: undefined,
		onCreate:
			typeof config.onCreate === "function"
				? wrapNativeCallback(
						async (
							error: unknown,
							payload: {
								ctx: NativeActorContext;
								input?: Buffer;
							},
						): Promise<void> => {
							const { ctx, input } = unwrapTsfnPayload(error, payload);
							const actorCtx = makeActorCtx(ctx);
							try {
								await config.onCreate(actorCtx, decodeValue(input));
							} finally {
								await actorCtx.dispose();
							}
						},
					)
				: undefined,
		createVars:
			hasStaticVars || typeof config.createVars === "function"
				? wrapNativeCallback(
						async (
							error: unknown,
							payload: { ctx: NativeActorContext },
						): Promise<void> => {
							const { ctx } = unwrapTsfnPayload(error, payload);
							const actorCtx = makeActorCtx(ctx);
							try {
								const startedAt = performance.now();
								const vars = hasStaticVars
									? structuredClone(config.vars)
									: await config.createVars(actorCtx, undefined);
								logger().debug({
									msg: "perf user: createVarsMs",
									durationMs: performance.now() - startedAt,
								});
								actorCtx.vars = vars;
							} finally {
								await actorCtx.dispose();
							}
						},
					)
				: undefined,
		onMigrate:
			typeof config.onMigrate === "function" ||
			databaseProvider !== undefined
				? wrapNativeCallback(
						async (
							error: unknown,
							payload: {
								ctx: NativeActorContext;
								isNew: boolean;
							},
						) => {
							const { ctx, isNew } = unwrapTsfnPayload(
								error,
								payload,
							);
							const actorCtx = makeActorCtx(ctx);
							try {
								if (!isNew) {
									await actorCtx.closeDatabase(false);
								}
								await actorCtx.runDatabaseMigrations();
								if (typeof config.onMigrate === "function") {
									await config.onMigrate(actorCtx, isNew);
								}
							} catch (error) {
								await actorCtx.closeDatabase(true);
								throw error;
							} finally {
								await actorCtx.dispose();
							}
						},
					)
				: undefined,
		onWake:
			typeof config.onWake === "function"
				? wrapNativeCallback(
						async (
							error: unknown,
							payload: { ctx: NativeActorContext },
						) => {
							const { ctx } = unwrapTsfnPayload(error, payload);
							const actorCtx = makeActorCtx(ctx);
							try {
								const startedAt = performance.now();
								await config.onWake(actorCtx);
								logger().debug({
									msg: "perf user: onWakeMs",
									durationMs: performance.now() - startedAt,
								});
							} finally {
								await actorCtx.dispose();
							}
						},
					)
				: undefined,
		onBeforeActorStart:
			typeof config.onBeforeActorStart === "function"
				? wrapNativeCallback(
						async (
							error: unknown,
							payload: { ctx: NativeActorContext },
						) => {
							const { ctx } = unwrapTsfnPayload(error, payload);
							const actorCtx = makeActorCtx(ctx);
							try {
								await config.onBeforeActorStart(actorCtx);
							} finally {
								await actorCtx.dispose();
							}
						},
					)
				: undefined,
		onSleep:
			typeof config.onSleep === "function" ||
			databaseProvider !== undefined
				? wrapNativeCallback(
						async (
							error: unknown,
							payload: { ctx: NativeActorContext },
						) => {
							const { ctx } = unwrapTsfnPayload(error, payload);
							const actorCtx = makeActorCtx(ctx);
							try {
								if (typeof config.onSleep === "function") {
									await config.onSleep(actorCtx);
								}
							} finally {
								await actorCtx.dispose();
							}
						},
					)
				: undefined,
		onDestroy:
			typeof config.onDestroy === "function" ||
			databaseProvider !== undefined
				? wrapNativeCallback(
						async (error: unknown, payload: { ctx: NativeActorContext }) => {
							const { ctx } = unwrapTsfnPayload(error, payload);
							const actorCtx = makeActorCtx(ctx);
							try {
								if (typeof config.onDestroy === "function") {
									await config.onDestroy(actorCtx);
								}
							} finally {
								resolveNativeDestroy(ctx);
								await actorCtx.closeDatabase(true);
								await actorCtx.dispose();
							}
						},
					)
				: undefined,
		onBeforeConnect:
			typeof config.onBeforeConnect === "function"
				? wrapNativeCallback(
						async (
							error: unknown,
							payload: {
								ctx: NativeActorContext;
								params: Buffer;
								request?: {
									method: string;
									uri: string;
									headers?: Record<string, string>;
									body?: Buffer;
								};
							},
						) => {
							const { ctx, params, request } = unwrapTsfnPayload(
								error,
								payload,
							);
							const actorCtx = makeActorCtx(
								ctx,
								request ? buildRequest(request) : undefined,
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
						},
					)
				: undefined,
		createConnState:
			hasStaticConnState || hasDynamicConnState
				? wrapNativeCallback(
						async (
							error: unknown,
							payload: {
								ctx: NativeActorContext;
								conn: NativeConnHandle;
								params: Buffer;
								request?: {
									method: string;
									uri: string;
									headers?: Record<string, string>;
									body?: Buffer;
								};
							},
						): Promise<Buffer> => {
							const { ctx, conn, params, request } =
								unwrapTsfnPayload(error, payload);
							const actorCtx = makeActorCtx(
								ctx,
								request ? buildRequest(request) : undefined,
							);
							const connAdapter = new NativeConnAdapter(
								conn,
								schemaConfig,
								ctx,
								(connId) =>
									callNativeSync(() =>
										ctx.queueHibernationRemoval(connId),
									),
							);
							try {
								const nextConnState = hasStaticConnState
									? structuredClone(config.connState)
									: await config.createConnState(
											actorCtx,
											validateConnParams(
												schemaConfig.connParamsSchema,
												decodeValue(params),
											),
										);
								connAdapter.initializeState(nextConnState);
								return encodeValue(nextConnState);
							} finally {
								await actorCtx.dispose();
							}
						},
					)
				: undefined,
		onConnect:
			typeof config.onConnect === "function"
				? wrapNativeCallback(
						async (
							error: unknown,
							payload: {
								ctx: NativeActorContext;
								conn: NativeConnHandle;
								request?: {
									method: string;
									uri: string;
									headers?: Record<string, string>;
									body?: Buffer;
								};
							},
						) => {
							const { ctx, conn, request } = unwrapTsfnPayload(
								error,
								payload,
							);
							const actorCtx = makeActorCtx(
								ctx,
								request ? buildRequest(request) : undefined,
							);
							const connAdapter = new NativeConnAdapter(
								conn,
								schemaConfig,
								ctx,
								(connId) =>
									callNativeSync(() =>
										ctx.queueHibernationRemoval(connId),
									),
							);
							try {
								await config.onConnect(
									Object.assign(actorCtx, {
										conn: connAdapter,
									}),
									connAdapter,
								);
							} finally {
								await actorCtx.dispose();
							}
						},
					)
				: undefined,
		onDisconnectFinal:
			needsDisconnectCallback
				? wrapNativeCallback(
						async (
							error: unknown,
							payload: {
								ctx: NativeActorContext;
								conn: NativeConnHandle;
							},
						) => {
							const { ctx, conn } = unwrapTsfnPayload(
								error,
								payload,
							);
							const actorCtx = makeConnCtx(ctx, conn);
							try {
								// Core already removed the connection; this hook is
								// pure user dispatch.
								if (typeof config.onDisconnect === "function") {
									await config.onDisconnect(
										actorCtx,
										new NativeConnAdapter(
											conn,
											schemaConfig,
											ctx,
											(connId) =>
												callNativeSync(() =>
													ctx.queueHibernationRemoval(
														connId,
													),
												),
										),
									);
								}
							} finally {
								await actorCtx.dispose();
							}
						},
					)
				: undefined,
		onBeforeSubscribe:
			schemaConfig.events &&
			Object.values(schemaConfig.events).some(
				(schema) =>
					typeof (schema as { canSubscribe?: unknown })
						.canSubscribe === "function",
			)
				? wrapNativeCallback(
						async (
							error: unknown,
							payload: {
								ctx: NativeActorContext;
								conn: NativeConnHandle;
								eventName: string;
							},
						) => {
							const { ctx, conn, eventName } = unwrapTsfnPayload(
								error,
								payload,
							);
							const actorCtx = makeConnCtx(ctx, conn);
							try {
								const canSubscribe = getEventCanSubscribe(
									schemaConfig.events,
									eventName,
								);
								if (!canSubscribe) {
									return;
								}
								const result = await canSubscribe(actorCtx);
								if (typeof result !== "boolean") {
									throw new Error(
										"canSubscribe must return a boolean",
									);
								}
								if (!result) {
									throw forbiddenError();
								}
							} finally {
								await actorCtx.dispose();
							}
						},
					)
				: undefined,
		onBeforeActionResponse:
			typeof config.onBeforeActionResponse === "function"
				? wrapNativeCallback(
						async (
							error: unknown,
							payload: {
								ctx: NativeActorContext;
								name: string;
								args: Buffer;
								output: Buffer;
							},
						) => {
							const { ctx, name, args, output } =
								unwrapTsfnPayload(error, payload);
							const actorCtx = makeActorCtx(ctx);
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
						},
					)
				: undefined,
		onRequest: wrapNativeCallback(
			async (
				error: unknown,
				payload: {
					ctx: NativeActorContext;
					request: {
						method: string;
						uri: string;
						headers?: Record<string, string>;
						body?: Buffer;
					};
					cancelToken?: NativeCancellationToken;
				},
			) => {
				try {
					const { ctx, request, cancelToken } = unwrapTsfnPayload(
						error,
						payload,
					);
					const jsRequest = buildRequest(request);
					const inspectorResponse =
						await maybeHandleNativeInspectorRequest(
							ctx,
							request,
							jsRequest,
						);
					if (inspectorResponse) {
						return await toJsHttpResponse(inspectorResponse);
					}

					if (typeof config.onRequest !== "function") {
						return await toJsHttpResponse(
							new Response(null, { status: 404 }),
						);
					}

					const rawConnParams =
						jsRequest.headers.get(HEADER_CONN_PARAMS);
					let requestCtx:
						| ReturnType<typeof withConnContext>
						| undefined;
					let conn: NativeConnHandle | undefined;
					try {
						const connParams = validateConnParams(
							schemaConfig.connParamsSchema,
							rawConnParams
								? JSON.parse(rawConnParams)
								: undefined,
						);
						conn = await callNative(() =>
							ctx.connectConn(encodeValue(connParams), request),
						);
						requestCtx = makeConnCtx(
							ctx,
							conn,
							jsRequest,
							cancelToken,
						);
						const response = await config.onRequest(
							requestCtx,
							jsRequest,
						);
						if (!(response instanceof Response)) {
							throw new Error(
								"onRequest handler must return a Response",
							);
						}
						return await toJsHttpResponse(response);
					} catch (error) {
						const encodingHeader =
							jsRequest.headers.get(HEADER_ENCODING);
						const encoding: Encoding =
							encodingHeader === "cbor" ||
							encodingHeader === "bare"
								? encodingHeader
								: "json";
						const path = new URL(jsRequest.url).pathname;
						return await toJsHttpResponse(
							buildNativeRequestErrorResponse(
								encoding,
								path,
								error,
							),
						);
					} finally {
						await requestCtx?.dispose();
						if (conn) {
							await conn.disconnect();
						}
					}
				} catch (error) {
					logger().error({
						msg: "native onRequest failed",
						error,
					});
					throw error;
				}
			},
		),
		onWebSocket:
			typeof config.onWebSocket === "function"
				? wrapNativeCallback(
						async (
							error: unknown,
							payload: {
								ctx: NativeActorContext;
								ws: NativeWebSocket;
								request?: {
									method: string;
									uri: string;
									headers?: Record<string, string>;
									body?: Buffer;
								};
							},
						) => {
							const { ctx, ws, request } =
								unwrapTsfnPayload(error, payload);
							const jsRequest = request
								? buildRequest(request)
								: undefined;
							const actorCtx = makeActorCtx(ctx, jsRequest);
							try {
								await config.onWebSocket(
									actorCtx,
									new TrackedNativeWebSocketAdapter(
										actorCtx,
										new NativeWebSocketAdapter(ws),
									),
								);
							} finally {
								await actorCtx.dispose();
							}
						},
					)
				: undefined,
		run: (() => {
			const run = getRunFunction(config.run);
			if (!run) {
				return undefined;
			}

			return wrapNativeCallback(
				async (
					error: unknown,
					payload: { ctx: NativeActorContext },
				) => {
					const { ctx } = unwrapTsfnPayload(error, payload);
					const actorId = callNativeSync(() => ctx.actorId());
					const actorCtx = makeActorCtx(ctx);
					nativeRunHandlerActiveByActorId.set(actorId, true);
					try {
						await run(actorCtx);
					} finally {
						nativeRunHandlerActiveByActorId.set(actorId, false);
						await actorCtx.dispose();
					}
				},
			);
		})(),
		getWorkflowHistory:
			getRunInspectorConfig(config.run) !== undefined
				? wrapNativeCallback(
						async (
							error: unknown,
							payload: { ctx: NativeActorContext },
						) => {
							const { ctx } = unwrapTsfnPayload(error, payload);
							const history =
								getNativeWorkflowInspector(ctx)?.getHistory();
							return history == null
								? undefined
								: encodeValue(history);
						},
					)
				: undefined,
		replayWorkflow:
			getRunInspectorConfig(config.run) !== undefined
				? wrapNativeCallback(
						async (
							error: unknown,
							payload: {
								ctx: NativeActorContext;
								entryId?: string;
							},
						) => {
							const { ctx, entryId } = unwrapTsfnPayload(
								error,
								payload,
							);
							const workflowInspector =
								getNativeWorkflowInspector(ctx);
							if (!workflowInspector?.replayFromStep) {
								return undefined;
							}

							const history =
								(await workflowInspector.replayFromStep(
									entryId,
								)) ?? null;
							return history == null
								? undefined
								: encodeValue(history);
						},
					)
				: undefined,
		actions: Object.fromEntries(
			Object.entries(actionHandlers).map(([name, handler]) => [
				name,
				wrapNativeCallback(
					async (
						error: unknown,
						payload: {
							ctx: NativeActorContext;
							conn: NativeConnHandle | null;
							name: string;
							args: Buffer;
							cancelToken?: NativeCancellationToken;
						},
					) => {
						const { ctx, conn, args, cancelToken } = unwrapTsfnPayload(
							error,
							payload,
						);
						const actorCtx =
							conn != null
								? makeConnCtx(ctx, conn, undefined, cancelToken)
								: makeActorCtx(ctx, undefined, cancelToken);
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
					},
				),
			]),
		),
		onQueueSend: wrapNativeCallback(
			async (
				error: unknown,
				payload: {
					ctx: NativeActorContext;
					conn: NativeConnHandle;
					request: {
						method: string;
						uri: string;
						headers?: Record<string, string>;
						body?: Buffer;
					};
					name: string;
					body: Buffer;
					wait: boolean;
					timeoutMs?: bigint | number;
					cancelToken?: NativeCancellationToken;
				},
			) => {
				const {
					ctx,
					conn,
					request,
					name,
					body,
					wait,
					timeoutMs,
					cancelToken,
				} = unwrapTsfnPayload(error, payload);
				const jsRequest = buildRequest(request);
				const actorCtx = withConnContext(
					bindings,
					ctx,
					conn,
					createClient,
					schemaConfig,
					databaseProvider,
					jsRequest,
					stateEnabled,
					onStateChange,
					cancelToken,
				);
				try {
					if (
						!schemaConfig.queues ||
						!hasSchemaConfigKey(schemaConfig.queues, name)
					) {
						return { status: "completed" };
					}

					const canPublish = getQueueCanPublish(
						schemaConfig.queues,
						name,
					);
					if (canPublish && !(await canPublish(actorCtx))) {
						throw forbiddenError();
					}

					const decodedBody = decodeValue(body);
					if (wait) {
						try {
							const response = await actorCtx.queue.enqueueAndWait(
								name,
								decodedBody,
								{
									timeout:
										timeoutMs === undefined ||
										timeoutMs === null
											? undefined
											: Number(timeoutMs),
								},
							);
							return {
								status: "completed",
								response:
									response === undefined
										? undefined
										: encodeValue(response),
							};
						} catch (error) {
							if (
								(error as { group?: string; code?: string })
									.group === "queue" &&
								(error as { group?: string; code?: string })
									.code === "timed_out"
							) {
								return { status: "timedOut" };
							}
							throw error;
						}
					}

					await actorCtx.queue.send(name, decodedBody);
					return { status: "completed" };
				} finally {
					await actorCtx.dispose();
				}
			},
		),
		serializeState: wrapNativeCallback(
			async (
				error: unknown,
				payload: {
					ctx: NativeActorContext;
					reason: SerializeStateReason;
				},
			) => {
				const { ctx, reason } = unwrapTsfnPayload(error, payload);
				const actorCtx = makeActorCtx(ctx);
				try {
					return actorCtx.serializeForTick(reason);
				} finally {
					await actorCtx.dispose();
				}
			},
		),
	};

	return new bindings.NapiActorFactory(
		callbacks,
		buildActorConfig(definition, registryConfig),
	);
}

async function buildServeConfig(
	config: RegistryConfig,
): Promise<JsServeConfig> {
	if (!config.endpoint) {
		throw nativeEndpointNotConfiguredError();
	}

	const serveConfig: JsServeConfig = {
		version: config.envoy.version,
		endpoint: config.endpoint,
		token: config.token,
		namespace: config.namespace,
		poolName: config.envoy.poolName,
		handleInspectorHttpInRuntime: true,
		serverlessBasePath: config.serverless.basePath,
		serverlessPackageVersion: VERSION,
		serverlessClientEndpoint: config.publicEndpoint,
		serverlessClientNamespace: config.publicNamespace,
		serverlessClientToken: config.publicToken,
		serverlessValidateEndpoint: config.validateServerlessEndpoint,
		serverlessMaxStartPayloadBytes: config.serverless.maxStartPayloadBytes,
	};

	if (config.startEngine) {
		const { getEnginePath } = await loadEngineCli();
		serveConfig.engineBinaryPath = getEnginePath();
	}

	return serveConfig;
}

export async function buildNativeRegistry(config: RegistryConfig): Promise<{
	bindings: NativeBindings;
	registry: NativeCoreRegistry;
	serveConfig: JsServeConfig;
}> {
	if (
		config.test?.enabled &&
		process.env._RIVET_TEST_INSPECTOR_TOKEN === undefined
	) {
		process.env._RIVET_TEST_INSPECTOR_TOKEN = "token";
	}

	const bindings = await loadNativeBindings();
	const registry = new bindings.CoreRegistry();

	for (const [name, definition] of Object.entries(config.use)) {
		registry.register(
			name,
			buildNativeFactory(bindings, config, definition),
		);
	}

	return {
		bindings,
		registry,
		serveConfig: await buildServeConfig(config),
	};
}
