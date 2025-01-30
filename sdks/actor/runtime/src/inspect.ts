import { safeStringify } from "@rivet-gg/actor-common/utils";
import type { Metadata } from "@rivet-gg/actor-core";
import type * as wsToClient from "@rivet-gg/actor-protocol/ws/to_client";
import type { Context } from "hono";
import type { WSEvents } from "hono/ws";
import type { AnyActor, ExtractActorState } from "./actor";
import { type ActorConfig, mergeActorConfig } from "./config";
import { Connection } from "./connection";
import * as errors from "./errors";
import { handleMessageEvent } from "./event";
import { inspectLogger } from "./log";
import type { Rpc } from "./rpc";
import { throttle } from "./utils";

/**
 * Internal symbol used to mark a property as an inspection method.
 * @internal
 */
export const INSPECT_SYMBOL = Symbol("inspect");

export type ConnectionId = number;

type SkipFirst<T extends unknown[]> = T extends [infer _, ...infer Rest]
	? Rest
	: never;

function connectionMap<A extends AnyActor>() {
	let id: ConnectionId = 0;
	const map = new Map<ConnectionId, Connection<A>>();
	return {
		create: (
			...params: SkipFirst<ConstructorParameters<typeof Connection<A>>>
		) => {
			const conId = id++;
			const connection = new Connection<A>(conId, ...params);
			map.set(conId, connection);
			return connection;
		},
		delete: (conId: ConnectionId) => {
			map.delete(conId);
		},
		get: (conId: ConnectionId) => {
			return map.get(conId);
		},
		[Symbol.iterator]: () => map.values(),
		get size() {
			return map.size;
		},
	};
}

interface InspectionAccessProxy<A extends AnyActor> {
	connections: () => Iterable<Connection<A>>;
	state: () => { enabled: boolean; state: unknown };
	rpcs: () => string[];
	setState: (state: ExtractActorState<A>) => Promise<void> | void;
	onRpcCall: (ctx: Rpc<A>, rpc: string, args: unknown[]) => void;
}

/**
 * Thin compatibility layer for handling inspection access to an actor.
 * @internal
 */
export class ActorInspection<A extends AnyActor> {
	readonly #connections = connectionMap<A>();
	readonly #proxy: InspectionAccessProxy<A>;

	readonly #config: ActorConfig;

	readonly #metadata: Metadata;

	readonly #logger = inspectLogger();

	constructor(
		config: ActorConfig,
		metadata: Metadata,
		proxy: InspectionAccessProxy<A>,
	) {
		this.#config = mergeActorConfig(config);
		this.#metadata = metadata;
		this.#proxy = proxy;
	}

	readonly notifyStateChanged = throttle(async () => {
		const inspectionResult = this.inspect();
		this.#broadcast("_state-changed", inspectionResult.state);
	}, 500);

	readonly notifyConnectionsChanged = throttle(async () => {
		const inspectionResult = this.inspect();
		this.#broadcast("_connections-changed", inspectionResult.connections);
	}, 500);

	handleWebsocketConnection(c: Context): WSEvents<WebSocket> {
		let connection: Connection<A> | undefined;
		return {
			onOpen: (evt, ws) => {
				connection = this.#connections.create(
					ws,
					"cbor",
					undefined,
					false,
				);
			},
			onMessage: async (evt, ws) => {
				if (!connection) {
					this.#logger.warn("`connection` does not exist");
					return;
				}

				await handleMessageEvent(
					evt,
					this.#metadata,
					connection,
					this.#config,
					{
						onExecuteRpc: async (ctx, name, args) => {
							return await this.#executeRpc(ctx, name, args);
						},
						onSubscribe: async () => {
							// we do not support granular subscriptions
						},
						onUnsubscribe: async () => {
							// we do not support granular subscriptions
						},
					},
				);
			},
			onClose: () => {
				if (!connection) {
					this.#logger.warn("`connection` does not exist");
					return;
				}

				this.#connections.delete(connection.id);
			},
			onError: (error) => {
				this.#logger.warn("inspect websocket error", { error });
			},
		};
	}

	#broadcast(event: string, ...args: unknown[]) {
		if (this.#connections.size === 0) {
			return;
		}

		for (const connection of this.#connections) {
			connection.send(event, ...args);
		}
	}

	/**
	 * Safely transforms the actor state into a string for debugging purposes.
	 */
	#inspectState(): string {
		try {
			return safeStringify(this.#proxy.state().state, 128 * 1024 * 1024);
		} catch (error) {
			return JSON.stringify({ _error: new errors.StateTooLarge() });
		}
	}

	/**
	 * Public RPC method that inspects the actor's state and connections.
	 * @internal
	 * @returns The actor's state and connections.
	 */
	inspect(): wsToClient.InspectRpcResponse {
		return {
			// Filter out internal 'inspect' RPC
			rpcs: this.#proxy.rpcs(),
			state: {
				enabled: this.#proxy.state().enabled,
				native: this.#proxy.state().enabled ? this.#inspectState() : "",
			},
			connections: [...this.#proxy.connections()].map((connection) =>
				connection[INSPECT_SYMBOL](),
			),
		};
	}

	async #executeRpc(ctx: Rpc<A>, name: string, args: unknown[]) {
		if (name === "inspect") {
			return this.inspect();
		}

		if (name === "setState") {
			const state = args[0] as Record<string, unknown>;
			await this.#proxy.setState(state as ExtractActorState<A>);
			return;
		}

		return this.#proxy.onRpcCall(ctx, name, args);
	}
}
