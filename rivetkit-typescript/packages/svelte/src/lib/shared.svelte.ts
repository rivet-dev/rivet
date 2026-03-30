/**
 * Shared RivetKit helpers for provider-level setup and mixed reactive/raw usage.
 *
 * These helpers codify the shared-client patterns common in SvelteKit apps:
 * one transport, one RivetKit wrapper, many reactive and raw consumers.
 *
 * @module
 */

import type {
	ActorOptions,
	AnyActorRegistry,
	CreateRivetKitOptions,
} from "@rivetkit/framework-base";
import type {
	ActorConn,
	ActorConnStatus,
	AnyActorDefinition,
	Client,
	ExtractActorsFromRegistry,
} from "rivetkit/client";
import { extract } from "./internal/extract.js";
import type { MaybeGetter } from "./internal/types.js";
import { createRivetKitWithClient, type RivetKit } from "./rivetkit.svelte.js";

/** Lazily create and reuse a single RivetKit wrapper for a shared client factory. */
export function createSharedRivetKit<Registry extends AnyActorRegistry>(
	getClient: () => Client<Registry>,
	opts?: CreateRivetKitOptions<Registry>,
): () => RivetKit<Registry> {
	let rivet: RivetKit<Registry> | null = null;

	return () => {
		if (!rivet) {
			rivet = createRivetKitWithClient(getClient(), opts);
		}
		// biome-ignore lint/style/noNonNullAssertion: Guarded by `if (!rivet)` check above
		return rivet!;
	};
}

/**
 * Merge static actor options with static or reactive params.
 *
 * Useful for auth tokens and Svelte-derived params while keeping actor config
 * assembly declarative and easy to reuse.
 */
export function withActorParams<
	Registry extends AnyActorRegistry,
	ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
>(
	base: MaybeGetter<ActorOptions<Registry, ActorName>>,
	params: MaybeGetter<Record<string, unknown> | undefined>,
): () => ActorOptions<Registry, ActorName> {
	return () => {
		const resolvedBase = extract(base);
		const resolvedParams = extract(params);
		const mergedParams = {
			...(resolvedBase.params ?? {}),
			...(resolvedParams ?? {}),
		};

		return {
			...resolvedBase,
			...(Object.keys(mergedParams).length > 0
				? { params: mergedParams }
				: {}),
		};
	};
}

export interface ReactiveConnectionSource {
	connect(): ActorConn<AnyActorDefinition>;
}

export interface ReactiveConnection {
	readonly connection: ActorConn<AnyActorDefinition> | null;
	readonly connStatus: ActorConnStatus;
	readonly error: Error | null;
	readonly isConnected: boolean;
	connect(): ActorConn<AnyActorDefinition>;
	disconnect(): Promise<void>;
	dispose(): Promise<void>;
	onEvent(
		eventName: string,
		handler: (...args: unknown[]) => void,
	): () => void;
}

/**
 * Create a reactive wrapper around an existing raw Rivet connection source.
 *
 * This is intended for low-level `handle.connect()` consumers that still want a
 * Svelte-friendly `connStatus` / `error` bridge without adopting `useActor`.
 */
export function createReactiveConnection(
	source: ReactiveConnectionSource,
): ReactiveConnection {
	let _connection = $state<ActorConn<AnyActorDefinition> | null>(null);
	let _connStatus = $state<ActorConnStatus>("idle");
	let _error = $state<Error | null>(null);

	const listeners = new Set<{
		eventName: string;
		handler: (...args: unknown[]) => void;
		unsubscribe?: () => void;
	}>();

	let cleanupStatus: (() => void) | null = null;
	let cleanupError: (() => void) | null = null;

	function bindConnection(conn: ActorConn<AnyActorDefinition>): void {
		cleanupStatus?.();
		cleanupError?.();

		_connection = conn;
		_connStatus = conn.connStatus;
		_error = null;

		cleanupStatus = conn.onStatusChange((status) => {
			_connStatus = status;
			if (status === "connected") {
				_error = null;
			}
		});

		cleanupError = conn.onError((error) => {
			_error = error instanceof Error ? error : new Error(String(error));
		});

		for (const listener of listeners) {
			listener.unsubscribe?.();
			listener.unsubscribe = conn.on(
				listener.eventName,
				listener.handler,
			);
		}
	}

	function connect(): ActorConn<AnyActorDefinition> {
		if (_connection) return _connection;
		const conn = source.connect();
		bindConnection(conn);
		return conn;
	}

	async function disconnect(): Promise<void> {
		const conn = _connection;
		if (!conn) return;

		cleanupStatus?.();
		cleanupStatus = null;
		cleanupError?.();
		cleanupError = null;

		for (const listener of listeners) {
			listener.unsubscribe?.();
			listener.unsubscribe = undefined;
		}

		await conn.dispose();
		_connection = null;
		_connStatus = "disconnected";
	}

	return {
		get connection() {
			return _connection;
		},
		get connStatus() {
			return _connStatus;
		},
		get error() {
			return _error;
		},
		get isConnected() {
			return _connStatus === "connected";
		},
		connect,
		disconnect,
		dispose() {
			return disconnect();
		},
		onEvent(
			eventName: string,
			handler: (...args: unknown[]) => void,
		): () => void {
			const listener: {
				eventName: string;
				handler: (...args: unknown[]) => void;
				unsubscribe?: () => void;
			} = { eventName, handler };
			listeners.add(listener);

			if (_connection) {
				listener.unsubscribe = _connection.on(eventName, handler);
			}

			return () => {
				listener.unsubscribe?.();
				listeners.delete(listener);
			};
		},
	};
}
