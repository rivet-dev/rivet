import { pathToFileURL } from "node:url";
import { parentPort, workerData } from "node:worker_threads";
import {
	type AnyActorDefinition,
	isStaticActorDefinition,
} from "@/actor/definition";
import { RivetError } from "@/actor/errors";
import { getLogger } from "@/common/log";
import { isDynamicActorDefinition } from "@/dynamic/internal";
import { Registry } from "@/registry";
import { RegistryConfigSchema } from "@/registry/config";
import { buildNativeFactory, encodeValue } from "@/registry/native";
import { RemoteCoreRuntime } from "./child-runtime";
import { toBridgeErrorPayload } from "./errors";
import type {
	BridgeCtxMeta,
	BridgeWorkerData,
	ChildToHostMessage,
	HostToChildMessage,
} from "./protocol";

/**
 * Bridged actor child entrypoint. Runs inside a `node:worker_threads` Worker
 * spawned by the host bridge (`src/bridge/host.ts`), serves exactly one actor,
 * and exits when the host terminates the worker after sleep/destroy.
 *
 * The child resolves the actor definition (registry module import for
 * worker-runtime actors, loader-resolved source for dynamic actors), builds
 * the regular native factory glue against a RemoteCoreRuntime, and then
 * dispatches host callback envelopes into the captured callbacks bag.
 *
 * Production workers run the bundled `child-entry` and resolve definitions by
 * importing them at runtime. Development workers run an esbuild-prebundled
 * entry that statically imports the definition module and passes it through
 * `provided` (see `dev-bundle.ts`).
 */

function logger() {
	return getLogger("actor-bridge-child");
}

interface ChildInit extends BridgeWorkerData {
	ctxMeta: BridgeCtxMeta;
}

export interface ProvidedDefinitionSource {
	/** Exports of the worker registry module, statically bundled. */
	moduleExports?: Record<string, unknown>;
	/** Default export of the dynamic actor source, statically bundled. */
	sourceDefault?: unknown;
}

function resolveFromModuleExports(
	moduleExports: Record<string, unknown>,
	exportName: string | undefined,
	actorName: string,
	moduleLabel: string,
): AnyActorDefinition {
	const fromExport = (value: unknown): AnyActorDefinition | undefined => {
		if (value instanceof Registry) {
			const definition = (
				value.config.use as Record<string, AnyActorDefinition>
			)[actorName];
			if (definition) {
				return definition;
			}
		}
		if (
			value &&
			(isStaticActorDefinition(value as AnyActorDefinition) ||
				isDynamicActorDefinition(value as AnyActorDefinition))
		) {
			return value as AnyActorDefinition;
		}
		return undefined;
	};

	if (exportName) {
		const definition = fromExport(moduleExports[exportName]);
		if (!definition) {
			throw new Error(
				`export ${exportName} of ${moduleLabel} is not a registry containing actor ${actorName} or an actor definition`,
			);
		}
		return definition;
	}

	// Prefer a registry export so the definition matches what the host serves.
	for (const value of Object.values(moduleExports)) {
		if (value instanceof Registry) {
			const definition = fromExport(value);
			if (definition) {
				return definition;
			}
		}
	}
	// Fall back to a definition exported under the actor name.
	const named = fromExport(moduleExports[actorName]);
	if (named) {
		return named;
	}
	throw new Error(
		`module ${moduleLabel} does not export a registry containing actor ${actorName}`,
	);
}

function resolveSourceDefault(value: unknown): AnyActorDefinition {
	if (!value || !isStaticActorDefinition(value as AnyActorDefinition)) {
		throw new Error(
			"dynamic actor source must export an actor definition as its default export",
		);
	}
	return value as AnyActorDefinition;
}

async function resolveDefinition(
	init: ChildInit,
	provided: ProvidedDefinitionSource | undefined,
): Promise<AnyActorDefinition> {
	if (init.bootstrap.kind === "module") {
		if (provided?.moduleExports) {
			return resolveFromModuleExports(
				provided.moduleExports,
				init.bootstrap.exportName,
				init.actorName,
				init.bootstrap.module,
			);
		}
		const moduleExports = (await import(init.bootstrap.module)) as Record<
			string,
			unknown
		>;
		return resolveFromModuleExports(
			moduleExports,
			init.bootstrap.exportName,
			init.actorName,
			init.bootstrap.module,
		);
	}

	if (provided && "sourceDefault" in provided) {
		return resolveSourceDefault(provided.sourceDefault);
	}
	const moduleExports = (await import(
		pathToFileURL(init.bootstrap.sourcePath).href
	)) as Record<string, unknown>;
	return resolveSourceDefault(moduleExports.default);
}

function buildChildRegistryConfig(
	init: ChildInit,
	definition: AnyActorDefinition,
) {
	const parsed = RegistryConfigSchema.parse({
		use: { [init.actorName]: definition },
		test: { enabled: init.registryConfig.testEnabled },
		maxIncomingMessageSize: init.registryConfig.maxIncomingMessageSize,
		maxOutgoingMessageSize: init.registryConfig.maxOutgoingMessageSize,
		endpoint: init.registryConfig.endpoint,
		token: init.registryConfig.token,
		namespace: init.registryConfig.namespace,
		envoy: { poolName: init.registryConfig.poolName },
		headers: init.registryConfig.headers,
		startEngine: false,
	});
	// Public client fields are derived by the registry config transform on the
	// host; carry the host's resolved values instead of re-deriving them.
	parsed.publicEndpoint = init.registryConfig.publicEndpoint;
	parsed.publicNamespace = init.registryConfig.publicNamespace;
	parsed.publicToken = init.registryConfig.publicToken;
	return parsed;
}

export async function bootstrapBridgeChild(
	provided?: ProvidedDefinitionSource,
): Promise<void> {
	const port = parentPort;
	if (!port) {
		throw new Error("bridge child must run inside a worker thread");
	}
	const init = workerData as ChildInit;

	const send = (message: ChildToHostMessage) => {
		port.postMessage(message);
	};

	try {
		const definition = await resolveDefinition(init, provided);
		const runtime = new RemoteCoreRuntime(port, init.ctxMeta);
		const registryConfig = buildChildRegistryConfig(init, definition);
		buildNativeFactory(runtime, registryConfig, definition);
		const callbacks = runtime.callbacks;
		if (!callbacks) {
			throw new Error(
				"buildNativeFactory did not register callbacks with the remote runtime",
			);
		}

		port.on("message", (message: HostToChildMessage) => {
			switch (message.kind) {
				case "cb:invoke": {
					void dispatchCallback(runtime, callbacks, send, message);
					break;
				}
				case "rpc:result": {
					runtime.handleRpcResult(
						message.seq,
						message.ok,
						message.value,
						message.error,
					);
					break;
				}
				case "evt:websocket": {
					runtime.handleWebSocketEvent(message.wsId, message.event);
					break;
				}
				case "evt:tokenCancelled": {
					runtime.handleTokenCancelled(message.tokenId);
					break;
				}
				case "evt:abort": {
					runtime.handleAbort();
					break;
				}
				case "evt:postError": {
					logger().warn({
						msg: "fire-and-forget runtime call failed on host",
						method: message.method,
						error: message.error.message,
						group: message.error.group,
						code: message.error.code,
					});
					break;
				}
			}
		});

		port.on("close", () => {
			runtime.failPending("bridge port closed");
		});

		send({
			kind: "ready",
			callbackNames: Object.entries(callbacks)
				.filter(([, value]) => value !== undefined)
				.map(([name]) => name),
		});
	} catch (error) {
		logger().error({
			msg: "bridge child bootstrap failed",
			error: toBridgeErrorPayload(error).message,
		});
		send({ kind: "bootstrapError", error: toBridgeErrorPayload(error) });
	}
}

async function dispatchCallback(
	runtime: RemoteCoreRuntime,
	callbacks: Record<string, unknown>,
	send: (message: ChildToHostMessage) => void,
	message: Extract<HostToChildMessage, { kind: "cb:invoke" }>,
) {
	try {
		const payload = decodeCallbackPayload(runtime, message.payload);
		const value = await invokeCallback(
			callbacks,
			message.callback,
			message.actionName,
			payload,
		);
		// Conn lifecycle ends with onDisconnectFinal; release the child-side
		// stub so a long-lived actor does not accumulate conn mirrors.
		if (message.callback === "onDisconnectFinal") {
			const conn = payload.conn as { connId?: string } | undefined;
			if (conn?.connId) {
				runtime.releaseConn(conn.connId);
			}
		}
		send({ kind: "cb:result", seq: message.seq, ok: true, value });
	} catch (error) {
		send({
			kind: "cb:result",
			seq: message.seq,
			ok: false,
			error: toBridgeErrorPayload(error),
		});
	}
}

async function invokeCallback(
	callbacks: Record<string, unknown>,
	callbackName: string,
	actionName: string | undefined,
	payload: Record<string, unknown>,
): Promise<unknown> {
	if (callbackName === "action") {
		if (!actionName) {
			throw new Error("action dispatch envelope is missing actionName");
		}
		const actions = callbacks.actions as
			| Record<string, (...args: unknown[]) => unknown>
			| undefined;
		const handler = actions?.[actionName];
		if (!handler) {
			throw new RivetError(
				"actor",
				"action_not_found",
				`Action ${actionName} not found`,
				{ public: true },
			);
		}
		return await handler(null, payload);
	}

	const callback = callbacks[callbackName] as
		| ((...args: unknown[]) => unknown)
		| undefined;
	if (!callback) {
		// The host registers the full callback surface for dynamic actors
		// because the loaded definition is unknown at factory creation. A
		// missing user callback matches core's behavior for an unregistered
		// callback: no-op success, with state-producing callbacks returning
		// encoded undefined and response hooks passing the output through.
		if (
			callbackName === "createState" ||
			callbackName === "createConnState"
		) {
			return encodeValue(undefined);
		}
		if (callbackName === "onBeforeActionResponse") {
			return payload.output;
		}
		return undefined;
	}
	return await callback(null, payload);
}

function decodeCallbackPayload(
	runtime: RemoteCoreRuntime,
	payload: Record<string, unknown>,
): Record<string, unknown> {
	const decoded: Record<string, unknown> = {};
	for (const [key, value] of Object.entries(payload)) {
		decoded[key] = decodePayloadValue(runtime, value);
	}
	return decoded;
}

function decodePayloadValue(
	runtime: RemoteCoreRuntime,
	value: unknown,
): unknown {
	if (
		typeof value === "object" &&
		value !== null &&
		"__bridge" in value &&
		typeof (value as { __bridge?: unknown }).__bridge === "string"
	) {
		return runtime.decodeHandleRef(
			value as Parameters<RemoteCoreRuntime["decodeHandleRef"]>[0],
		);
	}
	return value;
}
