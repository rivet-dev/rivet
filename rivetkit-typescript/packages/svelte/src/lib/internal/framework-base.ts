/**
 * Package-local Rivet framework bridge.
 *
 * Derived from `@rivetkit/framework-base` 2.3.13 (Apache-2.0). Keeping this
 * small bridge in the package is intentional: the published framework-base
 * currently drops `getParams`, while actor credentials must be resolved again
 * for every initial connection and reconnect. A workspace-level package-manager
 * patch cannot provide that guarantee to downstream `@rivetkit/svelte` users.
 *
 * The reactivity core is a minimal imperative observable: one `Map` of actor
 * entries plus per-entry listener sets: replacing the upstream
 * `@tanstack/store` `Store`/`Derived`/`Effect` trio. Two reasons:
 *
 * 1. This package is Svelte-only. The adapter already bridges state into
 *    Svelte runes (`$state` slots fed by `applyState`), so the framework-
 *    agnostic interop machinery of a portable store bought nothing here.
 * 2. Svelte's own reactive collections (`SvelteMap`, `createSubscriber`)
 *    are deliberately NOT used for this registry: their reads register the
 *    surrounding Svelte effect as a subscriber and their writes re-run that
 *    effect body. A framework push must update `$state` slots via listeners
 *    WITHOUT re-running the `useActor()` effect (which would churn
 *    mount/unmount lifecycles). Plain, untracked objects are invisible to
 *    Svelte's dependency tracking: exactly the guarantee the adapter needs.
 *
 * Observable semantics preserved from the TanStack-based core:
 * - `getOrCreateActor` returns `{ key, mount, state }` where `state` exposes
 *   `.state` (current snapshot) and `.subscribe(cb)` delivering
 *   `{ currentVal }` payloads: the adapter consumes this shape unchanged.
 * - Option updates are deferred via `queueMicrotask` so nothing publishes
 *   framework state synchronously during a Svelte render.
 * - State writes notify listeners synchronously (matching the previous
 *   synchronous `setState` flush that `reconnect()`'s disable/enable
 *   microtask ordering depends on).
 * - The connect/dispose lifecycle transitions (the previous `Effect` body)
 *   run inline after each write, gated on `refCount > 0`: the TanStack
 *   `Effect` was only subscribed between the first mount and the cleanup
 *   timeout, so unmounted entries never self-connect.
 * - First mount connects synchronously when idle + enabled.
 * - Final unmount schedules a `setTimeout(0)` cleanup that disposes the
 *   connection and evicts the entry; a remount before it fires cancels it.
 */

import equal from "fast-deep-equal";
import type { AnyActorDefinition, Registry } from "rivetkit";
import type {
  ActorConn,
  ActorConnStatus,
  ActorHandle,
  Client,
  ExtractActorsFromRegistry,
} from "rivetkit/client";

export type AnyActorRegistry = Registry<any>;

export type { ActorConnStatus };

interface ActorStateReference {
  /** Identity hash generated from the actor options. */
  hash: string;
  /** Current typed actor handle, when one has been resolved. */
  handle: ActorHandle<AnyActorDefinition> | null;
  /** Current realtime actor connection. */
  connection: ActorConn<AnyActorDefinition> | null;
  /** Current connection lifecycle status. */
  connStatus: ActorConnStatus;
  /** Most recent connection error. */
  error: Error | null;
  /** Normalized options retained for reconnects. */
  opts: AnyActorOptions;
}

/** Options for one actor connection managed by the framework bridge. */
export interface ActorOptions<
  Registry extends AnyActorRegistry,
  ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
> {
  /** Typesafe actor name from the registry. */
  name: ActorName;
  /** Stable actor key. */
  key: string | string[];
  /** Static connection parameters. */
  params?: ExtractActorsFromRegistry<Registry>[ActorName]["params"];
  /**
   * Resolve parameters immediately before every connect or reconnect.
   * Prefer this for expiring credentials.
   */
  getParams?: () => Promise<unknown>;
  /** Region used only when creating a missing actor. */
  createInRegion?: string;
  /** Input used only when creating a missing actor. */
  createWithInput?: unknown;
  /** Whether this actor entry should hold a live connection. */
  enabled?: boolean;
  /** Resolve an existing actor without creating it. */
  noCreate?: boolean;
}

/**
 * Public snapshot shape consumed by the Svelte adapter: the previous core's
 * `Derived` value. A fresh immutable object is produced on every write.
 */
export type ActorStateSnapshot<
  Registry extends AnyActorRegistry,
  ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
> = Omit<ActorStateReference, "handle" | "connection"> & {
  handle: ActorHandle<ExtractActorsFromRegistry<Registry>[ActorName]> | null;
  connection: ActorConn<
    ExtractActorsFromRegistry<Registry>[ActorName]
  > | null;
  /** @deprecated Use `connStatus === "connected"` instead. */
  isConnected: boolean;
};

/**
 * Reactive-state handle returned from {@link getOrCreateActor}. Mirrors the
 * upstream `Derived` surface the adapter was built against: read the current
 * snapshot from `.state`, observe pushes through `.subscribe`.
 */
export interface ActorStateHandle<
  Registry extends AnyActorRegistry,
  ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
> {
  /** Current actor snapshot. Fresh object identity after every write. */
  readonly state: ActorStateSnapshot<Registry, ActorName>;
  /**
   * Observe snapshot pushes. The listener receives `{ currentVal }` (upstream
   * `Derived.subscribe` payload shape). Notifications are synchronous with
   * the state write.
   */
  subscribe(
    listener: (payload: {
      currentVal: ActorStateSnapshot<Registry, ActorName>;
    }) => void,
  ): () => void;
}

export type AnyActorOptions = ActorOptions<AnyActorRegistry, any>;

/** Framework bridge configuration. */
export interface CreateRivetKitOptions<Registry extends AnyActorRegistry> {
  /** Return the cache identity for one normalized actor option set. */
  hashFunction?: (opts: ActorOptions<Registry, any>) => string;
}

type ComputedActorState = ActorStateReference & {
  /** @deprecated Use `connStatus === "connected"` instead. */
  isConnected: boolean;
};

type StateListener = (payload: { currentVal: ComputedActorState }) => void;

/**
 * Fully-erased handle type for internal use. Applying the public
 * `ActorStateHandle<AnyActorRegistry, any>` forces evaluation of rivetkit's
 * `ExtractActorsFromRegistry` conditional types and trips TS2589: the same
 * instantiation-depth limit the Svelte adapter documents. The public generic
 * handle stays at the `createRivetKit` boundary via cast.
 */
type ErasedStateHandle = {
  readonly state: ComputedActorState;
  subscribe(listener: StateListener): () => void;
};

/**
 * Structural client type for internal use. Rivetkit's `Client<Registry>`
 * conditional types exceed TypeScript's instantiation depth limit when
 * erased to `Registry<any>` (TS2589): the same constraint the Svelte
 * adapter documents for its own internals. Only the two calls this bridge
 * makes are declared; the public `createRivetKit` signature stays fully
 * typed.
 */
type BridgeClient = {
  get(
    name: string,
    key: string | string[],
    opts: {
      params?: unknown;
      getParams?: () => Promise<unknown>;
    },
  ): { connect(): ActorConn<AnyActorDefinition> };
  getOrCreate(
    name: string,
    key: string | string[],
    opts: {
      params?: unknown;
      getParams?: () => Promise<unknown>;
      createInRegion?: string;
      createWithInput?: unknown;
    },
  ): { connect(): ActorConn<AnyActorDefinition> };
};

/**
 * One shared actor entry. The Map key is the identity hash; multiple
 * consumers of the same hash ref-count a single entry (and socket).
 */
type ActorEntry = {
  key: string;
  /** Current public snapshot. Replaced (never mutated) on every write. */
  state: ComputedActorState;
  /** Adapter-side push listeners (`useActor` / `createReactiveActor`). */
  listeners: Set<StateListener>;
  /** Active mounts. The entry (and its socket) is cleaned up at zero. */
  refCount: number;
  /** Pending zero-ref eviction timer; cleared by a remount. */
  cleanupTimeout: ReturnType<typeof setTimeout> | null;
  mount: () => () => void;
  stateHandle: ErasedStateHandle;
};

/** Create the ref-counted actor registry consumed by the Svelte adapter. */
export function createRivetKit<Registry extends AnyActorRegistry>(
  client: Client<Registry>,
  createOpts: CreateRivetKitOptions<Registry> = {},
) {
  const cache = new Map<string, ActorEntry>();
  const bridgeClient = client as unknown as BridgeClient;

  return {
    getOrCreateActor: <
      ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
    >(
      actorOpts: ActorOptions<Registry, ActorName>,
    ): {
      mount: () => () => void;
      state: ActorStateHandle<Registry, ActorName>;
      key: string;
    } =>
      getOrCreateActor(bridgeClient, createOpts as any, cache, actorOpts as AnyActorOptions) as {
        mount: () => () => void;
        state: ActorStateHandle<Registry, ActorName>;
        key: string;
      },
  };
}

function createStateHandle(entry: ActorEntry): ErasedStateHandle {
  return {
    get state() {
      return entry.state;
    },
    subscribe(listener: StateListener): () => void {
      entry.listeners.add(listener);
      return () => {
        entry.listeners.delete(listener);
      };
    },
  };
}

function getOrCreateActor(
  client: BridgeClient,
  createOpts: CreateRivetKitOptions<AnyActorRegistry>,
  cache: Map<string, ActorEntry>,
  actorOpts: AnyActorOptions,
): { mount: () => () => void; state: ErasedStateHandle; key: string } {
  const hash = createOpts.hashFunction ?? defaultHashFunction;
  const normalizedOpts = {
    ...actorOpts,
    enabled: actorOpts.enabled ?? true,
  } as AnyActorOptions;
  const key = hash(normalizedOpts);
  const existing = cache.get(key);

  if (!existing) {
    const entry: ActorEntry = {
      key,
      state: {
        hash: key,
        connStatus: "idle",
        connection: null,
        handle: null,
        error: null,
        opts: normalizedOpts,
        isConnected: false,
      },
      listeners: new Set(),
      refCount: 0,
      cleanupTimeout: null,
      mount: undefined as unknown as () => () => void,
      stateHandle: undefined as unknown as ActorStateHandle<any, any>,
    };
    entry.stateHandle = createStateHandle(entry);
    entry.mount = createMount(client, cache, key, entry);
    cache.set(key, entry);
    return { mount: entry.mount, state: entry.stateHandle, key };
  }

  if (!equal(existing.state.opts, normalizedOpts)) {
    // Avoid synchronously publishing framework state during a Svelte render.
    // The liveness guard skips the write if this entry was evicted (and
    // possibly replaced) before the microtask runs.
    queueMicrotask(() => {
      if (cache.get(key) !== existing) return;
      commitState(client, cache, key, existing, { opts: normalizedOpts });
    });
  }

  return { mount: existing.mount, state: existing.stateHandle, key };
}

function createMount(
  client: BridgeClient,
  cache: Map<string, ActorEntry>,
  key: string,
  entry: ActorEntry,
): () => () => void {
  return () => {
    if (entry.cleanupTimeout !== null) {
      clearTimeout(entry.cleanupTimeout);
      entry.cleanupTimeout = null;
    }

    entry.refCount += 1;
    if (entry.refCount === 1) {
      const state = entry.state;
      if (state.opts.enabled && state.connStatus === "idle") {
        // First mount connects synchronously: consumers (and the
        // getParams forwarding contract) rely on the client being called
        // before mount() returns.
        createConnection(client, cache, key, entry);
      }
    }

    let mounted = true;
    return () => {
      if (!mounted) return;
      mounted = false;
      entry.refCount -= 1;
      if (entry.refCount !== 0) return;

      entry.cleanupTimeout = setTimeout(() => {
        entry.cleanupTimeout = null;
        if (entry.refCount > 0) return;

        const connection = entry.state.connection;
        if (connection) void connection.dispose();
        cache.delete(key);
      }, 0);
    };
  };
}

/**
 * Write a partial state update: replace the snapshot, run the connect/dispose
 * lifecycle reconciliation (the previous core's `Effect` body), then notify
 * listeners. Reconciliation runs before notification so consumers never
 * observe a snapshot the lifecycle has already superseded.
 */
function commitState(
  client: BridgeClient,
  cache: Map<string, ActorEntry>,
  key: string,
  entry: ActorEntry,
  updates: Partial<ActorStateReference>,
): void {
  const next: ComputedActorState = { ...entry.state, ...updates };
  next.isConnected = next.connStatus === "connected";
  entry.state = next;
  reconcile(client, cache, key, entry);
  if (entry.listeners.size === 0) return;
  const snapshot = entry.state;
  for (const listener of [...entry.listeners]) {
    listener({ currentVal: snapshot });
  }
}

/**
 * Connect/dispose lifecycle transitions, run inline after every state write.
 * Mirrors the previous core's `Effect`: its subscription was bound to the
 * ref count (mounted between first mount and the cleanup timeout), so both
 * branches are gated on an active mount.
 */
function reconcile(
  client: BridgeClient,
  cache: Map<string, ActorEntry>,
  key: string,
  entry: ActorEntry,
): void {
  if (entry.refCount <= 0) return;

  const state = entry.state;
  if (!state.opts.enabled && state.connection) {
    void state.connection.dispose();
    commitState(client, cache, key, entry, {
      connection: null,
      handle: null,
      connStatus: "idle",
    });
    return;
  }

  if (state.connStatus === "idle" && state.opts.enabled) {
    queueMicrotask(() => {
      // The entry may have been evicted (or replaced under the same key)
      // since this transition was queued: a stale create would leak a
      // socket nobody references.
      if (cache.get(key) !== entry) return;
      const current = entry.state;
      if (
        entry.refCount > 0 &&
        current.connStatus === "idle" &&
        current.opts.enabled
      ) {
        createConnection(client, cache, key, entry);
      }
    });
  }
}

function createConnection(
  client: BridgeClient,
  cache: Map<string, ActorEntry>,
  key: string,
  entry: ActorEntry,
): void {
  // Evicted mid-flight (zero refs cleaned up while a queued create was
  // pending). Nothing references this entry: opening a socket would leak.
  if (cache.get(key) !== entry) return;

  const opts = entry.state.opts;

  commitState(client, cache, key, entry, {
    connStatus: "connecting",
    error: null,
  });

  try {
    const handle = opts.noCreate
      ? client.get(opts.name, opts.key, {
          params: opts.params,
          getParams: opts.getParams,
        })
      : client.getOrCreate(opts.name, opts.key, {
          params: opts.params,
          getParams: opts.getParams,
          createInRegion: opts.createInRegion,
          createWithInput: opts.createWithInput,
        });
    const connection = handle.connect();

    commitState(client, cache, key, entry, {
      handle: handle as ActorHandle<AnyActorDefinition>,
      connection: connection as ActorConn<AnyActorDefinition>,
    });

    connection.onStatusChange((status) => {
      // Stale-socket guard: a notification from a connection this entry has
      // since replaced (or dropped) must not overwrite newer state.
      if (cache.get(key) !== entry || entry.state.connection !== connection)
        return;
      commitState(client, cache, key, entry, {
        connStatus: status,
        ...(status === "connected" ? { error: null } : {}),
      });
    });

    connection.onError((error) => {
      if (cache.get(key) !== entry || entry.state.connection !== connection)
        return;
      commitState(client, cache, key, entry, { error });
    });
  } catch (error) {
    console.error("Failed to create actor connection", error);
    commitState(client, cache, key, entry, {
      connStatus: "disconnected",
      error: error instanceof Error ? error : new Error(String(error)),
    });
  }
}

function defaultHashFunction({
  name,
  key,
  params,
  noCreate,
}: AnyActorOptions): string {
  return JSON.stringify({ name, key, params, noCreate });
}
