/**
 * Package-local Rivet framework bridge.
 *
 * Derived from `@rivetkit/framework-base` 2.3.13 (Apache-2.0). Keeping this
 * small bridge in the package is intentional: the published framework-base
 * currently drops `getParams`, while actor credentials must be resolved again
 * for every initial connection and reconnect. A workspace-level package-manager
 * patch cannot provide that guarantee to downstream `@rivetkit/svelte` users.
 */

import { Derived, Effect, Store } from "@tanstack/store";
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

interface InternalRivetKitStore {
  actors: Record<string, ActorStateReference>;
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

export type ActorsStateDerived<
  Registry extends AnyActorRegistry,
  ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
> = Derived<
  Omit<ActorStateReference, "handle" | "connection"> & {
    handle: ActorHandle<ExtractActorsFromRegistry<Registry>[ActorName]> | null;
    connection: ActorConn<
      ExtractActorsFromRegistry<Registry>[ActorName]
    > | null;
    /** @deprecated Use `connStatus === "connected"` instead. */
    isConnected: boolean;
  }
>;

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

type ActorCacheEntry = {
  state: Derived<ComputedActorState>;
  key: string;
  mount: () => () => void;
  refCount: number;
  cleanupTimeout: ReturnType<typeof setTimeout> | null;
};

type ActorCache = Map<string, ActorCacheEntry>;

/** Create the ref-counted actor store consumed by the Svelte adapter. */
export function createRivetKit<Registry extends AnyActorRegistry>(
  client: Client<Registry>,
  createOpts: CreateRivetKitOptions<Registry> = {},
) {
  const store = new Store<InternalRivetKitStore>({ actors: {} });
  const cache: ActorCache = new Map();

  return {
    getOrCreateActor: <
      ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
    >(
      actorOpts: ActorOptions<Registry, ActorName>,
    ): {
      mount: () => () => void;
      state: ActorsStateDerived<Registry, ActorName>;
      key: string;
    } =>
      getOrCreateActor(client, createOpts, store, cache, actorOpts) as {
        mount: () => () => void;
        state: ActorsStateDerived<Registry, ActorName>;
        key: string;
      },
    store,
  };
}

function updateActor(
  store: Store<InternalRivetKitStore>,
  key: string,
  updates: Partial<ActorStateReference>,
): void {
  store.setState((previous) => ({
    ...previous,
    actors: {
      ...previous.actors,
      [key]: { ...previous.actors[key], ...updates },
    },
  }));
}

function getOrCreateActor<
  Registry extends AnyActorRegistry,
  ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
>(
  client: Client<Registry>,
  createOpts: CreateRivetKitOptions<Registry>,
  store: Store<InternalRivetKitStore>,
  cache: ActorCache,
  actorOpts: ActorOptions<Registry, ActorName>,
) {
  const hash = createOpts.hashFunction ?? defaultHashFunction;
  const normalizedOpts = {
    ...actorOpts,
    enabled: actorOpts.enabled ?? true,
  } as AnyActorOptions;
  const key = hash(normalizedOpts);
  const existing = store.state.actors[key];

  if (!existing) {
    store.setState((previous) => ({
      ...previous,
      actors: {
        ...previous.actors,
        [key]: {
          hash: key,
          connStatus: "idle",
          connection: null,
          handle: null,
          error: null,
          opts: normalizedOpts,
        },
      },
    }));
  } else if (!equal(existing.opts, normalizedOpts)) {
    // Avoid synchronously publishing framework state during a Svelte render.
    queueMicrotask(() => updateActor(store, key, { opts: normalizedOpts }));
  }

  const cached = cache.get(key);
  if (cached) return { ...cached, state: cached.state };

  const derived = new Derived({
    fn: ({ currDepVals: [currentStore] }) => {
      const actor = currentStore.actors[key];
      return {
        ...actor,
        isConnected: actor.connStatus === "connected",
      };
    },
    deps: [store],
  });

  const effect = new Effect({
    fn: () => {
      const actor = store.state.actors[key];
      if (!actor) {
        throw new Error(`Actor with key "${key}" is missing from the store`);
      }

      if (!actor.opts.enabled && actor.connection) {
        void actor.connection.dispose();
        updateActor(store, key, {
          connection: null,
          handle: null,
          connStatus: "idle",
        });
        return;
      }

      if (actor.connStatus === "idle" && actor.opts.enabled) {
        queueMicrotask(() => {
          const current = store.state.actors[key];
          if (current?.connStatus === "idle" && current.opts.enabled) {
            createConnection(client, store, key);
          }
        });
      }
    },
    deps: [derived],
  });

  let unsubscribeDerived: (() => void) | null = null;
  let unsubscribeEffect: (() => void) | null = null;

  const entry: ActorCacheEntry = {
    state: derived,
    key,
    refCount: 0,
    cleanupTimeout: null,
    mount: () => {
      if (entry.cleanupTimeout !== null) {
        clearTimeout(entry.cleanupTimeout);
        entry.cleanupTimeout = null;
      }

      entry.refCount += 1;
      if (entry.refCount === 1) {
        unsubscribeDerived = derived.mount();
        unsubscribeEffect = effect.mount();
        const actor = store.state.actors[key];
        if (actor?.opts.enabled && actor.connStatus === "idle") {
          createConnection(client, store, key);
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

          unsubscribeDerived?.();
          unsubscribeEffect?.();
          unsubscribeDerived = null;
          unsubscribeEffect = null;

          const actor = store.state.actors[key];
          if (actor?.connection) void actor.connection.dispose();
          store.setState((previous) => {
            const { [key]: _removed, ...actors } = previous.actors;
            return { ...previous, actors };
          });
          cache.delete(key);
        }, 0);
      };
    },
  };

  cache.set(key, entry);
  return { mount: entry.mount, state: derived, key };
}

function createConnection<Registry extends AnyActorRegistry>(
  client: Client<Registry>,
  store: Store<InternalRivetKitStore>,
  key: string,
): void {
  const actor = store.state.actors[key];
  if (!actor) {
    throw new Error(`Actor with key "${key}" is missing from the store`);
  }

  updateActor(store, key, { connStatus: "connecting", error: null });

  try {
    const handle = actor.opts.noCreate
      ? client.get(actor.opts.name, actor.opts.key, {
          params: actor.opts.params,
          getParams: actor.opts.getParams,
        })
      : client.getOrCreate(actor.opts.name, actor.opts.key, {
          params: actor.opts.params,
          getParams: actor.opts.getParams,
          createInRegion: actor.opts.createInRegion,
          createWithInput: actor.opts.createWithInput,
        });
    const connection = handle.connect();

    updateActor(store, key, {
      handle: handle as ActorHandle<AnyActorDefinition>,
      connection: connection as ActorConn<AnyActorDefinition>,
    });

    connection.onStatusChange((status) => {
      store.setState((previous) => {
        if (previous.actors[key]?.connection !== connection) return previous;
        return {
          ...previous,
          actors: {
            ...previous.actors,
            [key]: {
              ...previous.actors[key],
              connStatus: status,
              ...(status === "connected" ? { error: null } : {}),
            },
          },
        };
      });
    });

    connection.onError((error) => {
      store.setState((previous) => {
        if (previous.actors[key]?.connection !== connection) return previous;
        return {
          ...previous,
          actors: {
            ...previous.actors,
            [key]: { ...previous.actors[key], error },
          },
        };
      });
    });
  } catch (error) {
    console.error("Failed to create actor connection", error);
    updateActor(store, key, {
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
