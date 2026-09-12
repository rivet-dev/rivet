/**
 * @rivetkit/svelte — Svelte 5 runes integration for RivetKit actors.
 *
 * Bridges Rivet actor state into Svelte 5 reactive primitives (`$state`,
 * `$effect`) through the package-local ref-counted framework core.
 *
 * @module
 */

import {
  createRivetKit as createVanillaRivetKit,
  type ActorOptions,
  type AnyActorRegistry,
  type CreateRivetKitOptions,
} from "./internal/framework-base.js";
import {
  type Client,
  createClient,
  type ExtractActorsFromRegistry,
  type ActorConn,
  type ActorHandle,
  type ActorConnStatus,
  type AnyActorDefinition,
} from "rivetkit/client";
import { BROWSER, DEV } from "esm-env";
import type { MaybeGetter } from "./internal/types.js";
import { extract } from "./internal/extract.js";
import {
  createConnectionInspector,
  type ConnectionInspector,
} from "./connection-inspector.svelte.js";

export type { ActorConnStatus } from "./internal/framework-base.js";
export { createClient } from "rivetkit/client";
export type {
  ActorOptions,
  AnyActorRegistry,
} from "./internal/framework-base.js";

// ---------------------------------------------------------------------------
// Warm-up types
// ---------------------------------------------------------------------------

/**
 * Options for warming an actor without establishing a WebSocket connection.
 *
 * Only `name` and `key` are required — these identify the actor instance.
 * `createWithInput` is optional and only needed if the actor may not exist yet
 * and requires initialization data.
 *
 * Shared by both {@link RivetKit.warmUp | warmUp} (HTTP resolve, no WS) and
 * {@link RivetKit.preConnect | preConnect} (opens a real WS).
 *
 * @typeParam Registry - The actor registry type.
 * @typeParam ActorName - The specific actor name within the registry.
 */
export interface WarmUpActorOptions<
  Registry extends AnyActorRegistry = AnyActorRegistry,
  ActorName extends keyof ExtractActorsFromRegistry<Registry> & string =
    keyof ExtractActorsFromRegistry<Registry> & string,
> {
  /** Actor name in the registry. */
  name: ActorName;
  /** Compound key identifying the actor instance. */
  key: string | string[];
  /** If true, resolve only an existing actor and do not create it. */
  noCreate?: boolean;
  /**
   * Datacenter/region to create the actor in if it doesn't exist yet.
   * Existing actors stay in their current region; local dev drivers may ignore it.
   */
  createInRegion?: string;
  /** Optional initialization input (only used if actor doesn't exist yet). */
  createWithInput?: unknown;
}

/**
 * @deprecated Renamed to {@link WarmUpActorOptions}. Kept as an alias for
 * back-compat; will be removed in a future major.
 */
export type PreloadActorOptions<
  Registry extends AnyActorRegistry = AnyActorRegistry,
  ActorName extends keyof ExtractActorsFromRegistry<Registry> & string =
    keyof ExtractActorsFromRegistry<Registry> & string,
> = WarmUpActorOptions<Registry, ActorName>;

/**
 * Handle returned by {@link RivetKit.preConnect | preConnect}. The caller owns
 * the connection lifecycle — call {@link PreConnectHandle.dispose | dispose} to
 * tear down the WebSocket once it is no longer needed (or once a component has
 * taken over the connection).
 */
export interface PreConnectHandle {
  /** Tear down the pre-opened WebSocket connection. Safe to call more than once. */
  dispose: () => Promise<void>;
}

// Precomputed, immutable consumer-own knownProps Sets — these contain
// only the props that live on the consumer's `inner` object (mount,
// dispose, onEvent). Reactive state props are switched inline in the
// proxy.get trap, so they don't need to be in the Set.
const USE_ACTOR_OWN_PROPS: ReadonlySet<string> = new Set<string>(["onEvent"]);
const REACTIVE_ACTOR_OWN_PROPS: ReadonlySet<string> = new Set<string>([
  "mount",
  "dispose",
  "onEvent",
  "reconnect",
]);

// ---------------------------------------------------------------------------
// Action middleware types
// ---------------------------------------------------------------------------

/**
 * Configuration for action call middleware.
 *
 * When provided to `useActor` or `createReactiveActor` (via `actionDefaults`),
 * every proxied action call is wrapped with timeout, error capture, and
 * reactive loading state tracking.
 *
 * Inspired by TanStack Query's mutation options and Zod's safeParse pattern:
 * - Errors are captured to `lastActionError` reactive state by default
 * - `throwOnError` controls whether the promise also rejects (default: `false`)
 * - Lifecycle callbacks (`onActionStart`, `onActionSuccess`, etc.) fire
 *   at the definition level — always, regardless of component mount state
 */
export interface ActionDefaults {
  /**
   * Timeout in milliseconds for action calls.
   *
   * When an action exceeds this duration, the promise resolves to `undefined`
   * (or rejects if `throwOnError` is enabled) and `lastActionError` is set
   * to a timeout error. This bounds the local caller only; actor-side work
   * already in progress is not cancelled.
   *
   * Default: none (actions run until the actor responds or the connection
   * drops — Rivet's server-side `actionTimeout` is the ultimate backstop).
   */
  timeout?: number;
  /** Finite positive per-action deadlines; invalid entries fall back to timeout. */
  timeoutByAction?: Readonly<Record<string, number>>;

  /**
   * Controls whether action errors reject the returned promise.
   *
   * - `false` (default): Errors are captured to `lastActionError` reactive
   *   state. The promise resolves to `undefined`. This is the "safe" mode —
   *   no try/catch needed at the call site.
   * - `true`: Errors are captured to `lastActionError` AND re-thrown.
   *   The caller must handle the rejection.
   * - `(error, actionName) => boolean`: Called per-error to decide.
   *
   * Follows TanStack Query's mutation convention where reactive error state
   * is the primary error channel in UI frameworks.
   */
  throwOnError?: boolean | ((error: Error, actionName: string) => boolean);

  /**
   * Guard against calling actions while disconnected.
   *
   * When `true` (default), a lost connection (`disconnected` / error-like)
   * fails immediately. A first-paint handshake (`idle` / `connecting`) waits
   * for {@link ReactiveActorHandle.whenConnected} and then dispatches.
   * The readiness wait shares the configured action timeout deadline.
   */
  guardConnection?: boolean;

  /**
   * Called when any action call starts.
   *
   * Lifecycle callbacks are observational. If one throws, the returned action
   * promise rejects with that callback error after internal tracking is cleaned
   * up, regardless of `throwOnError`.
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  onActionStart?: (actionName: string, args: any[]) => void;
  /** Called when an action completes successfully. */
  onActionSuccess?: (actionName: string, data: unknown) => void;
  /** Called when an action fails (timeout, network, or actor error). */
  onActionError?: (error: Error, actionName: string) => void;
  /** Called after an action completes (success or failure). */
  onActionSettled?: (actionName: string) => void;
}

/**
 * Internal interceptor function type. Built from {@link ActionDefaults}
 * and applied by the stable recursive action proxy.
 *
 * @param actionName - The name of the actor action being called.
 * @param args - Arguments passed to the action.
 * @param call - The original action call (delegates to the live connection).
 * @returns The action result, or `undefined` if the error was swallowed.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type ActionInterceptor = (
  actionName: string,
  args: any[],
  call: (signal?: AbortSignal) => any,
) => Promise<any>;

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

/**
 * Proxied actor methods forwarded from the underlying connection at runtime.
 *
 * RivetKit's deeply nested conditional types inside `ActorConn` exceed
 * TypeScript's instantiation depth limit when
 * wrapped in `Omit`. This permissive index signature preserves the
 * "call any actor action on the object" DX while avoiding TS2589.
 * All reactive state properties above remain fully typed.
 *
 * **Note:** Actor method calls are untyped at the package level due to
 * this TypeScript constraint. Consumers should use typed client interfaces
 * from their actor registry for type safety at the call site.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type ProxiedActorMethods = Record<string, (...args: any[]) => any>;

/**
 * Reactive action tracking state, available when `actionDefaults` is configured.
 *
 * These properties are `$state`-backed — reads in `$derived`, `$effect`,
 * or template expressions are automatically tracked by Svelte 5.
 */
interface ActionTrackingState {
  /** `true` when any action call is in-flight. */
  readonly isMutating: boolean;
  /** Number of concurrent in-flight action calls. */
  readonly pendingActions: number;
  /** Most recent action error. Cleared on next successful action or {@link resetActionState}. */
  readonly lastActionError: Error | null;
  /** Name of the last action that was called. */
  readonly lastAction: string | null;
  /** Clear `lastActionError` and `lastAction` (return to clean state). */
  resetActionState(): void;
}

/**
 * Reactive actor state returned by {@link RivetKit.useActor | useActor}.
 *
 * All actor actions (e.g. `sendMessage`, `getState`) are available directly
 * on the object via Proxy forwarding to the underlying connection.
 *
 * Every property is backed by Svelte 5 `$state` — reads inside
 * `$derived` / `$effect` / template expressions are automatically tracked.
 *
 * @typeParam Registry - The actor registry type.
 * @typeParam ActorName - The specific actor name within the registry.
 */
export type ActorState<
  Registry extends AnyActorRegistry = AnyActorRegistry,
  ActorName extends keyof ExtractActorsFromRegistry<Registry> & string =
    keyof ExtractActorsFromRegistry<Registry> & string,
> = {
  /** The active WebSocket connection, or `null` when not connected. */
  readonly connection: ActorConn<AnyActorDefinition> | null;
  /** The actor handle used to create the connection. */
  readonly handle: ActorHandle<AnyActorDefinition> | null;
  /** Current connection lifecycle status (`"idle"` | `"connecting"` | `"connected"` | `"reconnecting"` | `"disconnected"`). */
  readonly connStatus: ActorConnStatus;
  /** Last connection error, or `null`. */
  readonly error: Error | null;
  /** Most recent non-null connection error observed for this actor. */
  readonly lastError: Error | null;
  /** `true` when `connStatus === "connected"`. */
  readonly isConnected: boolean;
  /** `true` once this actor has connected successfully at least once. */
  readonly hasEverConnected: boolean;
  /** Internal hash identifying this actor instance. */
  readonly hash: string;
  /**
   * Subscribe to a named event broadcast by the actor.
   *
   * The subscription is automatically cleaned up when the component unmounts.
   * Must be called during component initialization (alongside `useActor`).
   *
   * @param eventName - The event name to listen for.
   * @param handler - Callback invoked when the event fires.
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  onEvent: (eventName: string, handler: (...args: any[]) => void) => void;
  /**
   * Returns a promise that resolves to `true` when the actor connects,
   * or `false` if the timeout elapses first.
   *
   * Resolves immediately if already connected. Eliminates the need for
   * manual `setInterval` polling loops when waiting for connection readiness.
   *
   * @param timeout - Maximum time to wait in milliseconds (default: 30000).
   */
  whenConnected: (timeout?: number) => Promise<boolean>;
} & ActionTrackingState &
  ProxiedActorMethods;

/**
 * Reactive actor handle returned by {@link RivetKit.createReactiveActor | createReactiveActor}.
 *
 * All actor actions are automatically available as methods via Proxy
 * forwarding to the underlying connection.
 *
 * @typeParam Registry - The actor registry type.
 * @typeParam ActorName - The specific actor name within the registry.
 */
export type ReactiveActorHandle<
  Registry extends AnyActorRegistry,
  ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
> = {
  /** The active WebSocket connection, or `null` when not connected. */
  readonly connection: ActorConn<AnyActorDefinition> | null;
  /** The actor handle used to create the connection. */
  readonly handle: ActorHandle<AnyActorDefinition> | null;
  /** Current connection lifecycle status. */
  readonly connStatus: ActorConnStatus;
  /** Last connection error, or `null`. */
  readonly error: Error | null;
  /** Most recent non-null connection error observed for this actor. */
  readonly lastError: Error | null;
  /** `true` when `connStatus === "connected"`. */
  readonly isConnected: boolean;
  /** `true` once this actor has connected successfully at least once. */
  readonly hasEverConnected: boolean;
  /** Internal hash identifying this actor instance. */
  readonly hash: string;
  /**
   * Start the connection lifecycle.
   *
   * Framework-base handles ref counting internally — multiple mounts
   * to the same actor share one WebSocket.
   *
   * @returns An unmount function to decrement the ref count.
   */
  mount(): () => void;
  /**
   * Clean up all event subscriptions and the framework-base state subscription.
   * Call this when the reactive actor is no longer needed.
   */
  dispose(): void;
  /**
   * Force a brand-new underlying connection, tearing down the current one even
   * when it is a half-open "zombie" socket still reporting `connected` (NAT/LB
   * idle cull, half-open TCP). Drives framework-base's `enabled` toggle —
   * disabling disposes the live connection and resets the actor to `idle`,
   * re-enabling re-creates from `idle` — so a stale socket is replaced rather
   * than reused (`getParams` re-runs, minting a fresh auth token).
   *
   * A plain `dispose()` + `mount()` cannot do this: framework-base only creates
   * a connection from `idle`, and a zombie still reports `connected`, so the dead
   * socket would be reused. No-op when the actor was never mounted.
   *
   * Event subscriptions registered via {@link onEvent} are automatically
   * re-bound onto the fresh connection.
   */
  reconnect(): void;
  /**
   * Subscribe to an actor broadcast event.
   *
   * Automatically re-binds when the connection changes (e.g. after reconnect).
   *
   * @param eventName - The event name to listen for.
   * @param handler - Callback invoked when the event fires.
   * @returns An unsubscribe function.
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  onEvent(eventName: string, handler: (...args: any[]) => void): () => void;
  /**
   * Returns a promise that resolves to `true` when the actor connects,
   * or `false` if the timeout elapses first.
   *
   * Resolves immediately if already connected. Eliminates the need for
   * manual `setInterval` polling loops when waiting for connection readiness.
   *
   * @param timeout - Maximum time to wait in milliseconds (default: 30000).
   */
  whenConnected(timeout?: number): Promise<boolean>;
} & ActionTrackingState &
  ProxiedActorMethods;

// ---------------------------------------------------------------------------
// Proxy helper — wraps a getter-based inner object so unknown props
// forward to the live actor connection. Used by both useActor and
// createReactiveActor. Closure-based $state avoids the Proxy + private
// field incompatibility that exists with Svelte 5 class-field $state.
//
// When an interceptAction function is provided (built from actionDefaults),
// every proxied method call is wrapped with it — enabling timeout, error
// capture, and reactive loading state tracking without manual wrapping.
// ---------------------------------------------------------------------------

// The per-actor proxy is now built inside `createActorCoreState.createProxy`
// so its `get` trap reads `_connection` directly without going through an
// indirection function. This eliminates a function-call frame on every
// `actor.someProp` access.

// ---------------------------------------------------------------------------
// Action defaults merge helper
// ---------------------------------------------------------------------------

/**
 * Shallow-merge client-level and actor-level action defaults.
 * Actor-level values override client-level. `undefined` at actor level
 * does NOT clear a client-level value (use explicit `null` convention
 * if clearing is needed in the future).
 */
function mergeActionDefaults(
  clientLevel: ActionDefaults | undefined,
  actorLevel: ActionDefaults | undefined,
): ActionDefaults | undefined {
  if (!clientLevel && !actorLevel) return undefined;
  if (!clientLevel) return actorLevel;
  if (!actorLevel) return clientLevel;
  return { ...clientLevel, ...actorLevel };
}

// ---------------------------------------------------------------------------
// Public interface
// ---------------------------------------------------------------------------

/**
 * The main RivetKit instance — returned by {@link createRivetKit} and
 * {@link createRivetKitWithClient}.
 *
 * Provides two APIs for connecting to actors:
 * - {@link RivetKit.useActor | useActor} — component-scoped, `$effect`-managed lifecycle.
 * - {@link RivetKit.createReactiveActor | createReactiveActor} — manual lifecycle for singletons and ViewModels.
 *
 * @typeParam Registry - The actor registry type.
 */
export interface RivetKit<Registry extends AnyActorRegistry> {
  /**
   * Connect to an actor and receive reactive state with auto-proxied methods.
   *
   * Must be called during component initialization (inside `<script>`).
   * Lifecycle is managed automatically via `$effect`.
   *
   * Accepts a static options object or a `MaybeGetter` thunk for reactive args:
   *
   * @example
   * ```typescript
   * // Static
   * useActor({ name: 'counter', key: ['main'] })
   * // Reactive — re-subscribes when roomId changes
   * useActor(() => ({ name: 'chatRoom', key: [roomId] }))
   * ```
   *
   * @param opts - Actor options or a getter returning actor options.
   * @returns A reactive, proxied object with actor state and methods.
   */
  useActor: <
    ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
  >(
    opts: MaybeGetter<SvelteActorOptions<Registry, ActorName>>,
  ) => ActorState<Registry, ActorName>;

  /**
   * Create a reactive actor handle with auto-proxied methods.
   *
   * Safe to call outside components (e.g. in a `.svelte.ts` module for
   * singletons). Lifecycle is manual via `mount()` / `dispose()`.
   * All actor actions are available directly on the returned object.
   *
   * @param opts - Actor options (name, key, params, etc.).
   * @returns A reactive, proxied handle with actor state, methods, and lifecycle controls.
   */
  createReactiveActor: <
    ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
  >(
    opts: SvelteActorOptions<Registry, ActorName>,
  ) => ReactiveActorHandle<Registry, ActorName>;

  /**
   * Warm an actor without establishing a WebSocket connection.
   *
   * Sends a single HTTP `PUT /actors` (getOrCreate + resolve) to ensure the
   * actor instance exists and is running. Useful for warming actors on hover
   * to eliminate cold-start latency on subsequent connections — similar to
   * SvelteKit's `data-sveltekit-preload-data` pattern.
   *
   * - **No WebSocket** — only an HTTP resolve call, no persistent connection.
   * - **Deduplicates** — same actor (name + key) is only resolved once per
   *   RivetKit instance. Failed attempts are removed from the dedup set so
   *   retries work.
   * - **Fire-and-forget** — errors are silently caught. Warm-up failure
   *   should never affect user experience.
   *
   * For the heavier tier that opens a real WebSocket ahead of time, see
   * {@link RivetKit.preConnect | preConnect}.
   *
   * @example
   * ```typescript
   * // Warm a document actor on hover
   * rivet.warmUp({ name: 'document', key: ['doc', docId] });
   * ```
   *
   * @param opts - Actor name and key to warm.
   */
  warmUp: <
    ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
  >(
    opts: WarmUpActorOptions<Registry, ActorName>,
  ) => void;

  /**
   * @deprecated Renamed to {@link RivetKit.warmUp | warmUp}. Kept as an alias
   * for back-compat; will be removed in a future major.
   */
  preloadActor: <
    ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
  >(
    opts: WarmUpActorOptions<Registry, ActorName>,
  ) => void;

  /**
   * Pre-open a real WebSocket connection to an actor ahead of time.
   *
   * The deliberate, heavier tier above {@link RivetKit.warmUp | warmUp}: this
   * opens (and keeps open) an actual connection so a subsequent component mount
   * finds the socket already connected. Use it only for high-intent signals —
   * broad hover should prefer `warmUp`, which costs a single HTTP call.
   *
   * - **Opens a WebSocket** — and keeps it open until you dispose it.
   * - **Caller owns the lifecycle** — you MUST call the returned
   *   `dispose()` when the connection is no longer needed, otherwise the
   *   socket leaks. (Connection sharing means a component that mounts the
   *   same actor reuses this socket.)
   * - **SSR-safe** — a no-op handle is returned during SSR.
   *
   * @example
   * ```typescript
   * const handle = rivet.preConnect({ name: 'chat', key: ['room', id] });
   * // later, once the user navigated away without entering the room:
   * await handle.dispose();
   * ```
   *
   * @param opts - Actor options (name, key, params, etc.).
   * @returns A handle whose `dispose()` tears the connection down.
   */
  preConnect: <
    ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
  >(
    opts: SvelteActorOptions<Registry, ActorName>,
  ) => PreConnectHandle;

  /**
   * Live registry of package-managed sockets. Present only when
   * {@link SvelteRivetKitOptions.connectionInspector} is `true`; otherwise
   * `null` and applyState does not report. Lists distinct sockets by
   * actor name + key (never params or tokens).
   */
  connectionInspector: ConnectionInspector | null;
}

// ---------------------------------------------------------------------------
// Factory functions
// ---------------------------------------------------------------------------

/**
 * Options for creating a RivetKit instance, extending framework-base options
 * with Svelte-specific action middleware defaults.
 */
export interface SvelteRivetKitOptions<
  Registry extends AnyActorRegistry,
> extends CreateRivetKitOptions<Registry> {
  /**
   * Default action middleware applied to all actors created by this instance.
   *
   * Per-actor `actionDefaults` (in `useActor`/`createReactiveActor` options)
   * shallow-merge on top of these client-level defaults.
   *
   * @example
   * ```typescript
   * const rivet = createRivetKit<AppRegistry>('http://localhost:3000', {
   *   actionDefaults: {
   *     timeout: 30_000,
   *     onActionError: (err, name) => errorTelemetry(name, err),
   *   },
   * });
   * ```
   */
  actionDefaults?: ActionDefaults;

  /**
   * Opt-in live connection registry for local-dev overlays.
   *
   * When `true`, the returned RivetKit exposes {@link RivetKit.connectionInspector}
   * and `useActor` / `createReactiveActor` / `preConnect` report status at
   * applyState. Off by default — production callers must not enable this.
   * The registry never records `params`, `getParams`, tokens, or payloads.
   */
  connectionInspector?: boolean;
}

/**
 * Actor options accepted by the Svelte adapter.
 *
 * Extends framework-base options with per-actor action middleware defaults.
 * Actor-level values shallow-merge over client-level defaults.
 */
export type SvelteActorOptions<
  Registry extends AnyActorRegistry,
  ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
> = ActorOptions<Registry, ActorName> & {
  actionDefaults?: ActionDefaults;
};

/**
 * Create a RivetKit instance with a new client.
 *
 * @param clientInput - Endpoint URL or client config passed to `createClient()`.
 * @param opts - Optional configuration including action middleware defaults.
 * @returns A {@link RivetKit} instance with `useActor` and `createReactiveActor`.
 *
 * @example
 * ```typescript
 * const rivet = createRivetKit<AppRegistry>('http://localhost:3000');
 *
 * // With client-level action defaults
 * const rivet = createRivetKit<AppRegistry>('http://localhost:3000', {
 *   actionDefaults: { timeout: 30_000, throwOnError: false },
 * });
 * ```
 */
export function createRivetKit<Registry extends AnyActorRegistry>(
  clientInput?: Parameters<typeof createClient<Registry>>[0],
  opts?: SvelteRivetKitOptions<Registry>,
): RivetKit<Registry> {
  return createRivetKitWithClient<Registry>(
    createClient<Registry>(clientInput),
    opts,
  );
}

/**
 * Create a RivetKit instance with a pre-existing client.
 *
 * @param client - An existing rivetkit `Client` instance.
 * @param opts - Optional configuration including action middleware defaults.
 * @returns A {@link RivetKit} instance with `useActor` and `createReactiveActor`.
 *
 * @example
 * ```typescript
 * import { createClient } from 'rivetkit/client';
 * const client = createClient<AppRegistry>('http://localhost:3000');
 * const rivet = createRivetKitWithClient<AppRegistry>(client);
 * ```
 */
export function createRivetKitWithClient<Registry extends AnyActorRegistry>(
  client: Client<Registry>,
  opts: SvelteRivetKitOptions<Registry> = {},
): RivetKit<Registry> {
  // Internal implementations erase the ActorName generic. The deeply nested
  // conditional types inside ActorConn (rivetkit 2.1.10) exceed TypeScript's
  // instantiation depth when evaluated in generic function bodies. The public
  // RivetKit<Registry> interface provides full type safety to consumers.
  const {
    actionDefaults: clientActionDefaults,
    connectionInspector: enableInspector = false,
    hashFunction: customHashFunction,
    ...baseFrameworkOpts
  } = opts;
  // `enabled` is lifecycle state, never actor identity. Normalizing it here
  // guarantees reconnect's enabled toggle targets the same framework entry,
  // even when a consumer supplies a custom hash function.
  const frameworkOpts: CreateRivetKitOptions<Registry> = customHashFunction
    ? {
        ...baseFrameworkOpts,
        hashFunction: (actorOpts) => {
          const { enabled: _enabled, ...identityOpts } = actorOpts;
          return customHashFunction(
            identityOpts as ActorOptions<
              Registry,
              keyof ExtractActorsFromRegistry<Registry> & string
            >,
          );
        },
      }
    : baseFrameworkOpts;
  // Strip the Svelte-only flag before handing opts to the framework core.
  const inspector = enableInspector ? createConnectionInspector() : null;
  let inspectorOwnerSeq = 0;
  function nextInspectorOwnerId(): string {
    inspectorOwnerSeq += 1;
    return `rivet-conn:${inspectorOwnerSeq}`;
  }
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const { getOrCreateActor } = createVanillaRivetKit<Registry>(
    client,
    frameworkOpts,
  ) as {
    getOrCreateActor: (actorOpts: any) => {
      key: string;
      mount: () => () => void;
      state: any;
    };
  };

  // -------------------------------------------------------------------
  // Action interceptor builder — creates a closure-based interceptor
  // that captures $state variables for reactive action tracking.
  //
  // The interceptor is called by the recursive action proxy for every
  // forwarded action call, providing: timeout, error capture to $state, loading
  // tracking, and lifecycle callbacks — without manual wrapping.
  // -------------------------------------------------------------------

  // Note: buildInterceptor was inlined into createActorCoreState so the
  // interceptor closes directly over the $state variables. Eliminates 6
  // getter/setter function calls per action and lets us collapse the promise
  // chain from .then().catch().finally() (3 ticks) into .then(ok, err) (1 tick).

  // -------------------------------------------------------------------
  // Shared actor core state — extracted from useActor and
  // createReactiveActor to eliminate the ~200-line duplication of
  // $state declarations, interceptor wiring, reactive getters,
  // applyState, and whenConnected.
  //
  // Returns a publicState prototype object (getter-based) and internal
  // helpers. Consumers use Object.create(core.publicState) to extend
  // with lifecycle-specific methods (onEvent, mount, dispose).
  // -------------------------------------------------------------------

  /**
   * Optional bridge from applyState into the opted-in connection inspector.
   * `isActive` gates reports so a disposed / unmounted handle cannot
   * re-register itself from a leftover derived subscription.
   */
  type InspectorBridge = {
    inspector: ConnectionInspector;
    ownerId: string;
    getIdentity: () => { name: string; key: string | string[] };
    isActive: () => boolean;
  };

  function createActorCoreState(
    actorActionDefaults: ActionDefaults | undefined,
    onConnectionChange?: (
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      prevConn: ActorConn<any> | null,
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      newConn: ActorConn<any> | null,
    ) => void,
    inspectorBridge?: InspectorBridge | null,
  ) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let _connection = $state.raw<ActorConn<any> | null>(null);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let _handle = $state.raw<ActorHandle<any> | null>(null);
    let _connStatus = $state<ActorConnStatus>("idle" as ActorConnStatus);
    let _error = $state.raw<Error | null>(null);
    let _lastError = $state.raw<Error | null>(null);
    let _hasEverConnected = $state(false);
    let _hash = $state("");

    // Action tracking state (active when actionDefaults is configured)
    let _isMutating = $state(false);
    let _pendingActions = $state(0);
    let _lastActionError = $state.raw<Error | null>(null);
    let _lastAction = $state<string | null>(null);

    // Non-reactive mirrors for action dispatch. Proxy-forwarded methods are
    // often invoked from effects; reading these mirrors avoids subscribing the
    // caller to connection/pending state while still keeping public getters
    // reactive through the `$state` slots above.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let connectionValue: ActorConn<any> | null = null;
    let connStatusValue: ActorConnStatus = "idle" as ActorConnStatus;
    let hasEverConnectedValue = false;
    let hashValue = "";
    let pendingActionsValue = 0;
    let stateGeneration = 0;
    const activeActionControllers = new Set<AbortController>();
    let actionInvocationSequence = 0;
    let latestActionInvocation = 0;
    // Installed by createProxy. Action invokers are rebound only when the
    // connection identity changes, keeping the per-call path branch-free.
    let updateProxyConnection:
      ((connection: ActorConn<any> | null) => void) | undefined;

    // whenConnected callback set — fired by applyState when connected.
    // Each callback accepts a boolean: true = connected, false = cancelled/disposed.
    const _onConnectedCallbacks = new Set<(connected: boolean) => void>();

    // Resolve action defaults: actor-level overrides client-level
    const resolvedDefaults = mergeActionDefaults(
      clientActionDefaults,
      actorActionDefaults,
    );

    // Inlined interceptor — closes over $state variables directly, avoiding
    // getter/setter dispatch per action invocation. The success path uses one
    // `.then(onSuccess, onError)` while cleanup remains exception-safe when a
    // consumer lifecycle callback throws.
    let interceptAction: ActionInterceptor | undefined;
    if (resolvedDefaults) {
      const defaults = resolvedDefaults;
      const guardOn = defaults.guardConnection !== false;
      const onStart = defaults.onActionStart;
      const onSuccess = defaults.onActionSuccess;
      const onError = defaults.onActionError;
      const onSettled = defaults.onActionSettled;
      const throwMode = defaults.throwOnError;
      const throwIsFn = typeof throwMode === "function";

      interceptAction = (
        actionName: string,
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        args: any[],
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        call: (signal?: AbortSignal) => any,
      ) => {
        const actionGeneration = stateGeneration;
        const actionInvocation = ++actionInvocationSequence;
        let actionController: AbortController | undefined;
        latestActionInvocation = actionInvocation;
        const override = defaults.timeoutByAction?.[actionName];
        const timeoutMs =
          typeof override === "number" &&
          Number.isFinite(override) &&
          override > 0
            ? override
            : (defaults.timeout ?? 0);
        const hasTimeout = Number.isFinite(timeoutMs) && timeoutMs > 0;
        const deadline = hasTimeout ? Date.now() + timeoutMs : 0;
        const ownsLatestActionState = (): boolean =>
          actionGeneration === stateGeneration &&
          actionInvocation === latestActionInvocation;
        const beginAction = (): Error | null => {
          pendingActionsValue += 1;
          _pendingActions = pendingActionsValue;
          _isMutating = true;
          _lastAction = actionName;
          return runCallback(
            onStart ? () => onStart(actionName, args) : undefined,
            null,
          );
        };

        const toError = (error: unknown): Error =>
          error instanceof Error ? error : new Error(String(error));

        const runCallback = (
          callback: (() => void) | undefined,
          previousError: Error | null,
        ): Error | null => {
          if (!callback) return previousError;
          try {
            callback();
            return previousError;
          } catch (error) {
            return previousError ?? toError(error);
          }
        };

        const finishPending = (
          timeoutId?: ReturnType<typeof setTimeout>,
        ): void => {
          if (timeoutId !== undefined) clearTimeout(timeoutId);
          if (actionController) {
            activeActionControllers.delete(actionController);
            actionController.abort();
          }
          // A disposed or identity-switched consumer owns a fresh state epoch;
          // a late action from the previous actor must not mutate it.
          if (actionGeneration !== stateGeneration) return;
          const newPending = Math.max(0, pendingActionsValue - 1);
          pendingActionsValue = newPending;
          _pendingActions = newPending;
          _isMutating = newPending > 0;
        };

        const rejectForCallbackOrPolicy = (
          err: Error,
          callbackError: Error | null,
        ): Promise<undefined> => {
          if (callbackError) return Promise.reject(callbackError);

          let shouldThrow: boolean;
          try {
            shouldThrow = throwIsFn
              ? (throwMode as (e: Error, n: string) => boolean)(err, actionName)
              : throwMode === true;
          } catch (error) {
            return Promise.reject(toError(error));
          }

          return shouldThrow ? Promise.reject(err) : Promise.resolve(undefined);
        };

        const settleFailure = (
          error: unknown,
          timeoutId?: ReturnType<typeof setTimeout>,
        ) => {
          const err = toError(error);
          if (ownsLatestActionState()) {
            _lastActionError = err;
          }
          finishPending(timeoutId);
          let callbackError = runCallback(
            onError ? () => onError(err, actionName) : undefined,
            null,
          );
          callbackError = runCallback(
            onSettled ? () => onSettled(actionName) : undefined,
            callbackError,
          );
          return rejectForCallbackOrPolicy(err, callbackError);
        };

        const failGuard = () => {
          const notYet =
            connStatusValue === "idle" || connStatusValue === "connecting";
          const err = Object.assign(
            new Error(
              `Action "${actionName}" called while ${notYet ? "not yet connected" : "disconnected"}`,
            ),
            {
              code: notYet ? "ACTOR_NOT_YET_CONNECTED" : "ACTOR_DISCONNECTED",
              connStatus: connStatusValue,
            },
          );
          if (ownsLatestActionState()) {
            _lastActionError = err;
            _lastAction = actionName;
          }
          finishPending();
          let callbackError = runCallback(
            onError ? () => onError(err, actionName) : undefined,
            null,
          );
          callbackError = runCallback(
            onSettled ? () => onSettled(actionName) : undefined,
            callbackError,
          );
          return rejectForCallbackOrPolicy(err, callbackError);
        };

        const startError = beginAction();
        if (startError) {
          if (ownsLatestActionState()) {
            _lastActionError = startError;
          }
          finishPending();
          const callbackError = runCallback(
            onSettled ? () => onSettled(actionName) : undefined,
            startError,
          );
          return Promise.reject(callbackError);
        }

        // Lost sockets fail immediately. A first-paint handshake
        // (idle/connecting) waits before dispatching, so actions called during
        // initial setup do not fail before their socket can become ready.
        if (guardOn && (!connectionValue || connStatusValue !== "connected")) {
          const notYet =
            connStatusValue === "idle" || connStatusValue === "connecting";
          if (!notYet) return failGuard();
          const waitMs = hasTimeout
            ? Math.min(Math.max(0, deadline - Date.now()), 30_000)
            : 30_000;
          return whenConnected(waitMs).then((ok) => {
            if (actionGeneration !== stateGeneration) {
              const err = Object.assign(
                new Error(
                  `Action "${actionName}" was cancelled because the actor identity changed`,
                ),
                { code: "ACTOR_IDENTITY_CHANGED" },
              );
              return settleFailure(err);
            }
            if (!ok || !connectionValue || connStatusValue !== "connected") {
              return failGuard();
            }
            return dispatchCall();
          });
        }

        return dispatchCall();

        function dispatchCall(): Promise<unknown> {
          if (hasTimeout && deadline <= Date.now()) {
            return settleFailure(
              new Error(
                `Action "${actionName}" timed out after ${timeoutMs}ms`,
              ),
            );
          }

          let callPromise: Promise<unknown>;
          try {
            const controller = new AbortController();
            actionController = controller;
            activeActionControllers.add(controller);
            const aborted = new Promise<never>((_, reject) => {
              controller.signal.addEventListener(
                "abort",
                () => reject(new Error("Actor action cancelled")),
                { once: true },
              );
            });
            // Dispatch can synchronously dispose the consumer or throw.
            // Register first; self-handle until the race owns the rejection.
            void aborted.catch(() => {});
            const returned = call(controller.signal);
            callPromise = Promise.race([Promise.resolve(returned), aborted]);
          } catch (error) {
            return settleFailure(error);
          }

          // Manual race against timeout. This avoids both `Promise.race`
          // (which adds an aggregator and at least one extra Promise
          // allocation) and the unhandled-rejection suppression
          // `callPromise.catch(noop)` that the race version needed for the
          // case where the timeout wins. Direct .then on callPromise inside
          // our deferred handles both resolution paths cleanly.
          //
          // We keep setTimeout registration EAGER (not behind a
          // queueMicrotask) so vitest's `vi.useFakeTimers()` +
          // `vi.advanceTimersByTime()` test pattern still works — the
          // timer must be visible to fake-timer advancement at the call
          // site, not on the next microtask.
          let raced: Promise<unknown>;
          let timeoutId: ReturnType<typeof setTimeout> | undefined;
          if (hasTimeout) {
            // Connection readiness and dispatch share one caller deadline.
            // Time spent waiting for the initial socket is not granted again
            // once the action reaches the wire.
            const remainingMs = Math.max(0, deadline - Date.now());
            raced = new Promise<unknown>((resolve, reject) => {
              let settled = false;
              timeoutId = setTimeout(() => {
                if (settled) return;
                settled = true;
                reject(
                  new Error(
                    `Action "${actionName}" timed out after ${timeoutMs}ms`,
                  ),
                );
              }, remainingMs);
              callPromise.then(
                (val) => {
                  if (settled) return;
                  settled = true;
                  clearTimeout(timeoutId);
                  resolve(val);
                },
                (err) => {
                  if (settled) return;
                  settled = true;
                  clearTimeout(timeoutId);
                  reject(err);
                },
              );
            });
          } else {
            raced = callPromise;
          }

          // Single `.then(onSuccess, onError)` = 1 microtask tick. We inline the
          // cleanup into both branches instead of using `.finally()` (which would
          // chain another promise).
          return raced.then(
            (result) => {
              if (ownsLatestActionState()) {
                _lastActionError = null;
              }
              finishPending(timeoutId);
              let callbackError = runCallback(
                onSuccess ? () => onSuccess(actionName, result) : undefined,
                null,
              );
              callbackError = runCallback(
                onSettled ? () => onSettled(actionName) : undefined,
                callbackError,
              );
              if (callbackError) {
                if (ownsLatestActionState()) {
                  _lastActionError = callbackError;
                }
                return Promise.reject(callbackError);
              }
              return result;
            },
            (error) => settleFailure(error, timeoutId),
          );
        }
      };
    }

    // Whether anyone is watching for connection changes. `useActor` never
    // passes one; `createReactiveActor` does for event-listener rebinding.
    // Branching once here avoids reading `_connection` and calling the
    // callback path on every subscribe push from useActor.
    const watchConnChange = onConnectionChange !== undefined;

    function syncInspector(): void {
      if (!inspectorBridge || !inspectorBridge.isActive()) return;
      const identity = inspectorBridge.getIdentity();
      const name = String(identity.name ?? "");
      if (!name) return;
      inspectorBridge.inspector.report({
        ownerId: inspectorBridge.ownerId,
        name,
        key: identity.key,
        hash: hashValue,
        connStatus: connStatusValue,
        hasConnection: connectionValue != null,
      });
    }

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    function applyState(val: any): void {
      if (!val) return;
      const prevConn = connectionValue;
      const nextConn = val.connection;
      const nextStatus = val.connStatus;
      const nextError = val.error;

      connectionValue = nextConn;
      connStatusValue = nextStatus;
      _connection = nextConn;
      _handle = val.handle;
      _connStatus = nextStatus;
      _error = nextError;

      if (prevConn !== nextConn) {
        updateProxyConnection?.(nextConn);
      }

      // Only write `_lastError` when we actually have a new error to record.
      // Saves a read-modify-write on every state push where val.error is null,
      // which is the common case once a connection is healthy.
      if (nextError != null) _lastError = nextError;

      // `_hasEverConnected` is a monotonic latch — once true, never reverts.
      // Skipping the read+coalesce after the first connection means later
      // reconnects don't keep re-triggering the $state setter.
      if (!hasEverConnectedValue && nextStatus === "connected") {
        hasEverConnectedValue = true;
        _hasEverConnected = true;
      }

      // Skip the `?? ""` allocation when hash is already a string.
      hashValue = val.hash != null ? val.hash : "";
      _hash = hashValue;

      // Notify connection change listeners (used by createReactiveActor
      // to rebind event listeners on reconnect). Gated by `watchConnChange`
      // so the `useActor` path avoids the branch + identity compare on
      // every push.
      if (watchConnChange && prevConn !== nextConn) {
        onConnectionChange!(prevConn, nextConn);
      }

      // Resolve pending whenConnected promises.
      // Snapshot first: a callback may synchronously call whenConnected(),
      // re-adding to the set. Clearing before iteration prevents the new
      // entry from being lost.
      if (nextStatus === "connected" && _onConnectedCallbacks.size > 0) {
        const snapshot = [..._onConnectedCallbacks];
        _onConnectedCallbacks.clear();
        for (const cb of snapshot) cb(true);
      }

      syncInspector();
    }

    /**
     * Promise-based ready signal. Resolves to `true` when the actor
     * connects, or `false` if the timeout elapses first.
     *
     * Resolves immediately if already connected.
     */
    function whenConnected(timeout = 30_000): Promise<boolean> {
      if (connStatusValue === "connected") return Promise.resolve(true);

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
    }

    /**
     * Cancel all pending whenConnected promises, resolving each with `false`.
     * Called during dispose() / effect cleanup to prevent leaked timers.
     */
    function cancelPendingConnections(): void {
      if (_onConnectedCallbacks.size === 0) return;
      const snapshot = [..._onConnectedCallbacks];
      _onConnectedCallbacks.clear();
      for (const cb of snapshot) cb(false);
    }

    /** Detach this consumer from the last framework state snapshot. */
    function detach(): void {
      const prevConn = connectionValue;
      stateGeneration += 1;
      for (const controller of activeActionControllers) controller.abort();
      activeActionControllers.clear();
      connectionValue = null;
      connStatusValue = "idle" as ActorConnStatus;
      hasEverConnectedValue = false;
      hashValue = "";
      pendingActionsValue = 0;
      _connection = null;
      _handle = null;
      _connStatus = "idle" as ActorConnStatus;
      _error = null;
      _lastError = null;
      _hasEverConnected = false;
      _hash = "";
      _pendingActions = 0;
      _isMutating = false;
      _lastActionError = null;
      _lastAction = null;
      updateProxyConnection?.(null);
      if (watchConnChange && prevConn) {
        onConnectionChange!(prevConn, null);
      }
    }

    // Stable, per-actor references to the two state-mutating helpers.
    // Hoisted out of `publicState` so the proxy.get switch can return them
    // without re-binding on every read.
    function resetActionState(): void {
      _lastActionError = null;
      _lastAction = null;
    }

    // Inline proxy factory — closes over every `$state` slot directly, so
    // each `actor.someProp` read becomes a single switch jump + signal read.
    // This collapses three former dispatch hops (proxy.get → target[prop] →
    // prototype getter call → $state read) into one. `inner` only carries
    // consumer-specific own props (`mount`, `dispose`, `onEvent`).
    function createProxy<T extends object>(
      inner: T,
      ownKnownProps: ReadonlySet<string>,
    ): T {
      // Action proxies are stable for the life of this consumer, matching the
      // stable mutation/send functions exposed by mature Svelte adapters. Each
      // proxy is rebound when connection identity changes, so destructured
      // handlers survive reconnects without a per-call identity comparison.
      const cachedMethods = new Map<string, unknown>();
      const connectionBinders = new Set<
        (connection: ActorConn<any> | null) => void
      >();

      updateProxyConnection = (connection) => {
        for (const bindConnection of connectionBinders) {
          bindConnection(connection);
        }
      };

      function getActionProxy(path: string[], actionName: string): unknown {
        const cached = cachedMethods.get(actionName);
        if (cached !== undefined) return cached;

        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        let resolvedInvoke: ((...args: any[]) => unknown) | null = null;

        const bindConnection = (conn: ActorConn<any> | null): void => {
          if (!conn) {
            resolvedInvoke = null;
            return;
          }

          let value: unknown = conn;
          for (const segment of path) {
            if (
              value == null ||
              (typeof value !== "object" && typeof value !== "function")
            ) {
              resolvedInvoke = null;
              return;
            }
            value = (value as Record<string, unknown>)[segment];
          }
          resolvedInvoke =
            typeof value === "function"
              ? (value as (...callArgs: unknown[]) => unknown)
              : null;
        };
        connectionBinders.add(bindConnection);
        bindConnection(connectionValue);

        const invokeCurrent = (...args: unknown[]): unknown => {
          const invoke = resolvedInvoke;
          if (!invoke) {
            const notYet =
              connStatusValue === "idle" || connStatusValue === "connecting";
            if (connectionValue) {
              return Promise.reject(
                new TypeError(`Actor action "${actionName}" is not callable`),
              );
            }
            return Promise.reject(
              Object.assign(
                new Error(
                  `Action "${actionName}" called while ${notYet ? "not yet connected" : "disconnected"}`,
                ),
                {
                  code: notYet
                    ? "ACTOR_NOT_YET_CONNECTED"
                    : "ACTOR_DISCONNECTED",
                  connStatus: connStatusValue,
                },
              ),
            );
          }

          // Rivet nested-action functions are Proxies: direct invocation is
          // required because `.apply` itself is interpreted as another path.
          return invoke(...args);
        };

        const callable = interceptAction
          ? (...args: unknown[]): unknown =>
              interceptAction(actionName, args, (signal) => {
                const conn = connectionValue;
                if (conn && typeof conn.action === "function") {
                  return conn.action({ name: actionName, args, signal });
                }
                return invokeCurrent(...args);
              })
          : invokeCurrent;

        const actionProxy = new Proxy(callable, {
          get(target, prop, receiver) {
            if (typeof prop !== "string") {
              return Reflect.get(target, prop, receiver);
            }
            // Match Rivet's own proxy and avoid Promise/await assimilation.
            if (prop === "then") return undefined;
            const nestedName = `${actionName}.${prop}`;
            return getActionProxy([...path, prop], nestedName);
          },
        });
        cachedMethods.set(actionName, actionProxy);
        return actionProxy;
      }

      return new Proxy(inner, {
        get(target, prop, receiver) {
          if (typeof prop !== "string") {
            return Reflect.get(target, prop, receiver);
          }

          // Hot path: inline reactive state reads. Order roughly matches
          // observed access frequency — connStatus/isConnected/connection
          // dominate template re-reads.
          switch (prop) {
            case "then":
              return undefined;
            case "connStatus":
              return _connStatus;
            case "isConnected":
              return _connStatus === "connected";
            case "connection":
              return _connection;
            case "isMutating":
              return _isMutating;
            case "error":
              return _error;
            case "lastError":
              return _lastError;
            case "handle":
              return _handle;
            case "hasEverConnected":
              return _hasEverConnected;
            case "hash":
              return _hash;
            case "pendingActions":
              return _pendingActions;
            case "lastActionError":
              return _lastActionError;
            case "lastAction":
              return _lastAction;
            case "resetActionState":
              return resetActionState;
            case "whenConnected":
              return whenConnected;
          }

          // Consumer-specific own props (mount/dispose/onEvent)
          if (ownKnownProps.has(prop)) {
            return (target as Record<string, unknown>)[prop];
          }

          const cachedAction = cachedMethods.get(prop);
          if (cachedAction !== undefined) return cachedAction;

          const conn = connectionValue;
          if (conn) {
            const val = (conn as unknown as Record<string, unknown>)[prop];
            if (typeof val !== "function") return val;
          }
          return getActionProxy([prop], prop);
        },
      });
    }

    return {
      applyState,
      syncInspector,
      cancelPendingConnections,
      detach,
      getConnection: () => _connection,
      createProxy,
    };
  }

  // -------------------------------------------------------------------
  // useActor — component-scoped, $effect-managed lifecycle
  //
  // Accepts static options or a MaybeGetter thunk for reactive args.
  // Returns a Proxy that forwards unknown props to the actor connection,
  // giving flat access to actor methods (e.g. actor.sendMessage()).
  //
  // When actionDefaults is provided (at actor or client level), every
  // proxied action call is wrapped with the interceptor for timeout,
  // error capture, and reactive loading state.
  // -------------------------------------------------------------------

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  function useActor(optsOrGetter: MaybeGetter<any>): any {
    // Resolve action defaults from the initial options (not reactive —
    // actionDefaults are structural config, not per-render state).
    const initialOpts = extract(optsOrGetter);
    const inspectorOwnerId = inspector ? nextInspectorOwnerId() : "";
    const identity = {
      name: String(initialOpts?.name ?? ""),
      key: (initialOpts?.key ?? []) as string | string[],
    };
    const core = createActorCoreState(
      initialOpts?.actionDefaults,
      undefined,
      inspector
        ? {
            inspector,
            ownerId: inspectorOwnerId,
            getIdentity: () => identity,
            isActive: () => true,
          }
        : null,
    );
    let activeFrameworkKey: string | null = null;
    let lifecycleRevision = 0;

    $effect(() => {
      const actorOpts = extract(optsOrGetter);
      identity.name = String(actorOpts?.name ?? "");
      identity.key = (actorOpts?.key ?? []) as string | string[];

      // Strip actionDefaults before passing to the framework core
      // (it doesn't know about our Svelte-specific extension)
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { actionDefaults: _ad, ...baseOpts } = actorOpts ?? {};

      const {
        key: nextFrameworkKey,
        mount,
        state: derived,
      } = getOrCreateActor(baseOpts);
      const unmount = mount();

      // Effect teardowns run both before a dependency refresh and on final
      // destruction. Defer final detachment by one microtask so a replacement
      // effect body can compare framework identity first. Same-hash option
      // refreshes retain action waiters and state; only a true actor identity
      // change starts a fresh state generation.
      lifecycleRevision += 1;
      if (
        activeFrameworkKey !== null &&
        activeFrameworkKey !== nextFrameworkKey
      ) {
        core.cancelPendingConnections();
        core.detach();
        inspector?.unregister(inspectorOwnerId);
      }
      activeFrameworkKey = nextFrameworkKey;

      core.applyState(derived.state);

      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const unsub = derived.subscribe(({ currentVal }: { currentVal: any }) =>
        core.applyState(currentVal),
      );

      return () => {
        unsub();
        unmount();
        const cleanupRevision = ++lifecycleRevision;
        queueMicrotask(() => {
          if (cleanupRevision !== lifecycleRevision) return;
          activeFrameworkKey = null;
          core.cancelPendingConnections();
          core.detach();
          inspector?.unregister(inspectorOwnerId);
        });
      };
    });

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    function onEvent(
      eventName: string,
      handler: (...args: any[]) => void,
    ): void {
      $effect(() => {
        const conn = core.getConnection();
        if (!conn) return;
        return conn.on(eventName, handler);
      });
    }

    // Reactive state is served by core.createProxy's inline switch, so the
    // inner object only carries the consumer-owned `onEvent` method.
    const inner = { onEvent };

    return core.createProxy<object>(inner, USE_ACTOR_OWN_PROPS);
  }

  // -------------------------------------------------------------------
  // warmUp — fire-and-forget actor warm-up via resolve()
  // -------------------------------------------------------------------

  /**
   * Set of actor hashes whose resolve request is currently in flight.
   * Keyed by a length-prefixed actor identity tuple so compound keys containing
   * separators cannot suppress unrelated warm-ups. Completed requests leave
   * the set so a later hover can wake an actor that has since gone back to sleep.
   */
  const _warming = new Set<string>();
  const _warmInputRefs = new WeakMap<object, number>();
  let _warmInputRefSeq = 0;

  function warmUpInputHash(value: unknown): string {
    try {
      const serialized = JSON.stringify(value);
      if (serialized !== undefined) return `json:${serialized}`;
    } catch {
      // Cyclic and BigInt-containing values may still be valid for Rivet's
      // encoder; fall through to an identity-safe, non-throwing hash.
    }

    if (
      (typeof value === "object" && value !== null) ||
      typeof value === "function"
    ) {
      const reference = value as object;
      let id = _warmInputRefs.get(reference);
      if (id === undefined) {
        id = ++_warmInputRefSeq;
        _warmInputRefs.set(reference, id);
      }
      return `ref:${id}`;
    }

    return `${typeof value}:${String(value)}`;
  }

  function warmUpHash(
    name: string,
    keyArray: string[],
    noCreate: boolean | undefined,
    createInRegion: string | undefined,
    createWithInput: unknown,
  ): string {
    let hash = `${name.length}:${name}`;
    for (const part of keyArray) {
      hash += `|${part.length}:${part}`;
    }
    if (noCreate) hash += "|noCreate";
    if (createInRegion !== undefined) {
      hash += `|region:${createInRegion.length}:${createInRegion}`;
    }
    return createWithInput === undefined
      ? hash
      : `${hash}|input:${warmUpInputHash(createWithInput)}`;
  }

  function warmUp(opts: WarmUpActorOptions): void {
    if (!BROWSER) return;

    const keyArray = Array.isArray(opts.key) ? opts.key : [opts.key];
    let hash: string | undefined;
    try {
      hash = warmUpHash(
        opts.name as string,
        keyArray,
        opts.noCreate,
        opts.createInRegion,
        opts.createWithInput,
      );
      if (_warming.has(hash)) return;
      _warming.add(hash);

      const accessor = (client as Record<string, unknown>)[
        opts.name as string
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
      ] as any;

      const handle = opts.noCreate
        ? accessor.get(keyArray)
        : accessor.getOrCreate(keyArray, {
            ...(opts.createInRegion !== undefined
              ? { createInRegion: opts.createInRegion }
              : {}),
            ...(opts.createWithInput !== undefined
              ? { createWithInput: opts.createWithInput }
              : {}),
          });

      void Promise.resolve(handle.resolve()).then(
        () => _warming.delete(hash!),
        () => _warming.delete(hash!),
      );
    } catch {
      // Warm-up is deliberately best-effort: hashing, accessor construction,
      // and resolve may all fail synchronously without affecting navigation.
      if (hash !== undefined) _warming.delete(hash);
    }
  }

  // -------------------------------------------------------------------
  // createReactiveActor — manual lifecycle, Proxy-forwarded methods
  //
  // When actionDefaults is provided (at actor or client level), every
  // proxied action call is wrapped with the interceptor for timeout,
  // error capture, and reactive loading state.
  // -------------------------------------------------------------------

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  function createReactiveActor(actorOpts: any): any {
    const _eventListeners = new Set<{
      event: string;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      handler: (...args: any[]) => void;
      unsubscribe?: () => void;
    }>();

    // Strip actionDefaults before passing to framework-base
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    const { actionDefaults: _ad, ...baseOpts } = actorOpts ?? {};
    let frameworkMount: (() => () => void) | null = null;
    let unsubscribeDerived: (() => void) | null = null;
    const activeUnmounts = new Set<() => void>();
    const inspectorOwnerId = inspector ? nextInspectorOwnerId() : "";

    // Create core state with connection-change callback for event rebinding
    const core = createActorCoreState(
      actorOpts?.actionDefaults,
      (_prevConn, newConn) => {
        for (const listener of _eventListeners) {
          if (listener.unsubscribe) listener.unsubscribe();
          listener.unsubscribe = undefined;
          if (newConn) {
            listener.unsubscribe = newConn.on(listener.event, listener.handler);
          }
        }
      },
      inspector
        ? {
            inspector,
            ownerId: inspectorOwnerId,
            getIdentity: () => ({
              name: String(actorOpts?.name ?? ""),
              key: (actorOpts?.key ?? []) as string | string[],
            }),
            isActive: () => activeUnmounts.size > 0,
          }
        : null,
    );

    function ensureFrameworkActor(): void {
      if (frameworkMount) return;
      const { mount, state: derived } = getOrCreateActor(baseOpts);
      frameworkMount = mount;
      core.applyState(derived.state);
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      unsubscribeDerived = derived.subscribe(
        ({ currentVal }: { currentVal: any }) => core.applyState(currentVal),
      );
    }

    // Reactive state is served by core.createProxy's inline switch, so the
    // inner object only carries the consumer-owned lifecycle methods.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const inner: Record<string, any> = {};

    inner.mount = () => {
      if (DEV && !BROWSER) {
        console.warn(
          "[@rivetkit/svelte] createReactiveActor.mount() called during SSR. " +
            "Mount should only be called in browser lifecycle (onMount, $effect, etc.).",
        );
      }
      ensureFrameworkActor();
      const frameworkUnmount = frameworkMount!();
      let called = false;
      const release = () => {
        if (called) return;
        called = true;
        activeUnmounts.delete(release);
        frameworkUnmount();
        if (activeUnmounts.size === 0) {
          // The framework removes an unowned cache entry after its grace
          // period. Release our binding too so a later mount reacquires a
          // live entry instead of mounting the removed entry's closure.
          unsubscribeDerived?.();
          unsubscribeDerived = null;
          frameworkMount = null;
          core.cancelPendingConnections();
          core.detach();
          inspector?.unregister(inspectorOwnerId);
        }
      };
      activeUnmounts.add(release);
      // First applyState runs inside ensureFrameworkActor before this
      // handle is marked active. Sync now that the mount ref exists.
      core.syncInspector();
      return release;
    };

    inner.dispose = () => {
      const unmounts = [...activeUnmounts];
      activeUnmounts.clear();
      for (const unmount of unmounts) unmount();
      unsubscribeDerived?.();
      unsubscribeDerived = null;
      frameworkMount = null;
      core.cancelPendingConnections();
      core.detach();
      inspector?.unregister(inspectorOwnerId);
      for (const listener of _eventListeners) {
        if (listener.unsubscribe) listener.unsubscribe();
        listener.unsubscribe = undefined;
      }
      _eventListeners.clear();
    };

    inner.reconnect = (): void => {
      // Nothing mounted yet → the first mount() will open a fresh connection
      // anyway, so there is nothing to replace.
      if (!frameworkMount) return;

      // Force framework-base to drop the current (possibly zombie) connection and
      // build a brand-new one through its own enabled→disabled→enabled machinery.
      // Disabling makes framework-base's effect call `connection.dispose()` and
      // reset the actor to "idle"; re-enabling re-creates from "idle". The hash
      // ignores `enabled`, so both calls target the SAME actor entry.
      //
      // Ordering: framework-base defers each opts change with `queueMicrotask`,
      // and its `commitState` flushes state writes to the entry and its
      // listeners SYNCHRONOUSLY, so by the time the re-enable microtask runs,
      // the disable has fully propagated (connection disposed, status "idle").
      // queueMicrotask FIFO, not timer ordering, is what guarantees the
      // re-enable observes "idle" and triggers create().
      getOrCreateActor({ ...baseOpts, enabled: false });
      queueMicrotask(() => {
        // Skip if the handle was disposed while the disable was in flight.
        if (!frameworkMount) return;
        getOrCreateActor({ ...baseOpts, enabled: true });
      });
    };

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    inner.onEvent = (
      eventName: string,
      handler: (...args: any[]) => void,
    ): (() => void) => {
      const listener: {
        event: string;
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        handler: (...args: any[]) => void;
        unsubscribe?: () => void;
      } = { event: eventName, handler };

      const conn = core.getConnection();
      if (conn) {
        listener.unsubscribe = conn.on(eventName, handler);
      }
      _eventListeners.add(listener);

      return () => {
        if (listener.unsubscribe) listener.unsubscribe();
        _eventListeners.delete(listener);
      };
    };

    return core.createProxy<object>(inner, REACTIVE_ACTOR_OWN_PROPS);
  }

  // -------------------------------------------------------------------
  // preConnect — open a real WebSocket ahead of time (caller-disposed)
  // -------------------------------------------------------------------

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  function preConnect(actorOpts: any): PreConnectHandle {
    // SSR-safe: no socket to open on the server. Return an inert handle so
    // call sites can `await handle.dispose()` unconditionally.
    if (!BROWSER) return { dispose: async () => {} };

    // `preConnect` is an explicit eager-connect request, so an inherited
    // `enabled: false` must not turn it into a silent no-op.
    const handle = createReactiveActor({ ...actorOpts, enabled: true });
    const unmount = handle.mount();
    let disposed = false;

    return {
      dispose: async () => {
        if (disposed) return;
        disposed = true;
        unmount();
        handle.dispose();
      },
    };
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return {
    useActor,
    createReactiveActor,
    warmUp,
    // Deprecated alias retained for back-compat with pre-rename call sites.
    preloadActor: warmUp,
    preConnect,
    connectionInspector: inspector,
  } as any;
}
