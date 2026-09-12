/**
 * Shared RivetKit helpers for provider-level setup and mixed reactive/raw usage.
 *
 * These helpers codify the shared-client patterns common in SvelteKit apps:
 * one transport, one RivetKit wrapper, many reactive and raw consumers.
 *
 * @module
 */

import type { AnyActorRegistry } from "./internal/framework-base.js";
import type {
  ActorConn,
  ActorConnStatus,
  AnyActorDefinition,
  Client,
  ExtractActorsFromRegistry,
} from "rivetkit/client";
import {
  createRivetKitWithClient,
  type RivetKit,
  type SvelteActorOptions,
  type SvelteRivetKitOptions,
} from "./rivetkit.svelte.js";
import type { MaybeGetter } from "./internal/types.js";
import { extract } from "./internal/extract.js";

/** Lazily create and reuse a single RivetKit wrapper for a shared client factory. */
export function createSharedRivetKit<Registry extends AnyActorRegistry>(
  getClient: () => Client<Registry>,
  opts?: SvelteRivetKitOptions<Registry>,
): () => RivetKit<Registry> {
  let rivet: RivetKit<Registry> | undefined;

  return () => {
    if (!rivet) {
      rivet = createRivetKitWithClient<Registry>(
        getClient(),
        opts,
      ) as RivetKit<Registry>;
    }
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
  base: MaybeGetter<SvelteActorOptions<Registry, ActorName>>,
  params: MaybeGetter<Record<string, unknown> | undefined>,
): () => SvelteActorOptions<Registry, ActorName> {
  return () => {
    const resolvedBase = extract(base);
    const resolvedParams = extract(params);
    const mergedParams = {
      ...(resolvedBase.params ?? {}),
      ...(resolvedParams ?? {}),
    };

    return {
      ...resolvedBase,
      ...(Object.keys(mergedParams).length > 0 ? { params: mergedParams } : {}),
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
  /** Open or return the current socket. */
  connect(): ActorConn<AnyActorDefinition>;
  /** Disconnect the socket while retaining event registrations for reconnect. */
  disconnect(): Promise<void>;
  /**
   * Backward-compatible alias for {@link ReactiveConnection.disconnect}.
   * Event registrations remain available for a later `connect()` call.
   */
  dispose(): Promise<void>;
  onEvent(eventName: string, handler: (...args: unknown[]) => void): () => void;
  /**
   * Returns a promise that resolves to `true` when the connection is
   * established, or `false` if the timeout elapses first.
   *
   * Resolves immediately if already connected.
   *
   * @param timeout - Maximum time to wait in milliseconds (default: 30000).
   */
  whenConnected(timeout?: number): Promise<boolean>;
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
  let _connection = $state.raw<ActorConn<AnyActorDefinition> | null>(null);
  let _connStatus = $state<ActorConnStatus>("idle");
  let _error = $state.raw<Error | null>(null);

  const listeners = new Set<{
    eventName: string;
    handler: (...args: unknown[]) => void;
    unsubscribe?: () => void | Promise<unknown>;
  }>();

  const _onConnectedCallbacks = new Set<(connected: boolean) => void>();

  /** Cancel all pending whenConnected promises, resolving each with the given value. */
  function cancelPendingConnections(connected: boolean): void {
    if (_onConnectedCallbacks.size === 0) return;
    const snapshot = [..._onConnectedCallbacks];
    _onConnectedCallbacks.clear();
    for (const cb of snapshot) cb(connected);
  }

  let cleanupStatus: (() => void) | null = null;
  let cleanupError: (() => void) | null = null;
  let disconnectPromise: Promise<void> | null = null;

  function bindConnection(conn: ActorConn<AnyActorDefinition>): void {
    cleanupStatus?.();
    cleanupError?.();

    _connection = conn;
    _connStatus = conn.connStatus;
    _error = null;
    if (_connStatus === "connected") {
      cancelPendingConnections(true);
    }

    cleanupStatus = conn.onStatusChange((status) => {
      _connStatus = status;
      if (status === "connected") {
        _error = null;
        cancelPendingConnections(true);
      }
    });

    cleanupError = conn.onError((error) => {
      _error = error instanceof Error ? error : new Error(String(error));
    });

    for (const listener of listeners) {
      listener.unsubscribe?.();
      // `ActorConn<AnyActorDefinition>` erases `on` at the type level (rivetkit
      // 2.3.13 omits it from the raw class and re-adds it only through the
      // definition-mapped event map). Runtime `on(event, handler)` is event
      // subscribe and returns an unsubscribe — cast the connection, not the
      // property, the same way `subscribe()` below does.
      const subscribe = (
        conn as unknown as {
          on: (
            eventName: string,
            handler: (payload: unknown) => void,
          ) => () => void | Promise<unknown>;
        }
      ).on;
      listener.unsubscribe = subscribe(listener.eventName, listener.handler);
    }
  }

  function connect(): ActorConn<AnyActorDefinition> {
    if (_connection) return _connection;
    const conn = source.connect();
    bindConnection(conn);
    return conn;
  }

  function disconnect(): Promise<void> {
    const conn = _connection;

    // A waiter may be registered before connect() is called. Disconnect still
    // settles that waiter even when there is no active transport yet.
    cancelPendingConnections(false);
    if (!conn) return disconnectPromise ?? Promise.resolve();

    // Detach synchronously before running cleanup or awaiting transport
    // teardown. A slow, rejected, or unexpectedly throwing close path must
    // never leave this socket reusable by a concurrent caller.
    _connection = null;
    _connStatus = "disconnected";
    _error = null;

    const teardownTasks: Promise<unknown>[] = [];
    const synchronousErrors: unknown[] = [];
    const runTeardown = (callback: (() => unknown) | null): void => {
      if (!callback) return;
      try {
        teardownTasks.push(Promise.resolve(callback()));
      } catch (error) {
        synchronousErrors.push(error);
      }
    };

    runTeardown(cleanupStatus);
    cleanupStatus = null;
    runTeardown(cleanupError);
    cleanupError = null;

    for (const listener of listeners) {
      runTeardown(listener.unsubscribe ?? null);
      listener.unsubscribe = undefined;
    }

    runTeardown(() => conn.dispose());

    const result = Promise.allSettled(teardownTasks).then((settlements) => {
      if (synchronousErrors.length > 0) throw synchronousErrors[0];
      const failed = settlements.find(
        (settlement): settlement is PromiseRejectedResult =>
          settlement.status === "rejected",
      );
      if (failed) throw failed.reason;
    });
    const pending = result.finally(() => {
      if (disconnectPromise === pending) disconnectPromise = null;
    });
    disconnectPromise = pending;
    return pending;
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
    dispose: disconnect,
    whenConnected(timeout = 30_000): Promise<boolean> {
      if (_connStatus === "connected") return Promise.resolve(true);

      return new Promise<boolean>((resolve) => {
        let settled = false;

        const cb = (connected: boolean) => {
          if (settled) return;
          settled = true;
          clearTimeout(timeoutId);
          _onConnectedCallbacks.delete(cb);
          resolve(connected);
        };

        _onConnectedCallbacks.add(cb);

        const timeoutId = setTimeout(() => {
          if (settled) return;
          settled = true;
          _onConnectedCallbacks.delete(cb);
          resolve(false);
        }, timeout);
      });
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
        // Cast: the actor action surface has grown large enough that
        // rivetkit's generic `on` overload resolution hits TypeScript's
        // recursion limit. The runtime shape is correct (string name,
        // handler callback) — we just erase the deep union at the type
        // boundary.
        listener.unsubscribe = (
          _connection as unknown as {
            on: (e: string, h: (...a: unknown[]) => void) => () => void;
          }
        ).on(eventName, handler);
      }

      return () => {
        listener.unsubscribe?.();
        listeners.delete(listener);
      };
    },
  };
}
