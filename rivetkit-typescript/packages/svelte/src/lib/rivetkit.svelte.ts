/**
 * @rivetkit/svelte — Svelte 5 runes integration for RivetKit actors.
 *
 * Thin adapter over `@rivetkit/framework-base` that bridges actor state
 * into Svelte 5 reactive primitives (`$state`, `$effect`).
 *
 * @module
 */

import {
	type ActorOptions,
	type AnyActorRegistry,
	type CreateRivetKitOptions,
	createRivetKit as createVanillaRivetKit,
} from "@rivetkit/framework-base";
import { BROWSER } from "esm-env";
import {
	type ActorConn,
	type ActorConnStatus,
	type ActorHandle,
	type AnyActorDefinition,
	type Client,
	createClient,
	type ExtractActorsFromRegistry,
} from "rivetkit/client";
import { extract } from "./internal/extract.js";
import type { MaybeGetter } from "./internal/types.js";

export type {
	ActorConnStatus,
	ActorOptions,
	AnyActorRegistry,
} from "@rivetkit/framework-base";
export { createClient } from "rivetkit/client";

// ---------------------------------------------------------------------------
// Preload types
// ---------------------------------------------------------------------------

/**
 * Options for preloading (warming) an actor without establishing a WebSocket
 * connection.
 *
 * Only `name` and `key` are required — these identify the actor instance.
 * `createWithInput` is optional and only needed if the actor may not exist yet
 * and requires initialization data.
 *
 * @typeParam Registry - The actor registry type.
 * @typeParam ActorName - The specific actor name within the registry.
 */
export interface PreloadActorOptions<
	Registry extends AnyActorRegistry = AnyActorRegistry,
	ActorName extends keyof ExtractActorsFromRegistry<Registry> &
		string = keyof ExtractActorsFromRegistry<Registry> & string,
> {
	/** Actor name in the registry. */
	name: ActorName;
	/** Compound key identifying the actor instance. */
	key: string | string[];
	/** Optional initialization input (only used if actor doesn't exist yet). */
	createWithInput?: unknown;
}

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
	 * to a timeout error.
	 *
	 * Default: none (actions run until the actor responds or the connection
	 * drops — Rivet's server-side `actionTimeout` is the ultimate backstop).
	 */
	timeout?: number;

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
	 * When `true` (default), actions called while the WebSocket connection is
	 * not established will immediately fail with a connection error instead of
	 * queuing or hanging.
	 */
	guardConnection?: boolean;

	/** Called when any action call starts. */
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
 * and passed to {@link proxyWithConnection}.
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
	call: () => Promise<any>,
) => Promise<any>;

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

/**
 * Proxied actor methods forwarded from the underlying connection at runtime.
 *
 * rivetkit 2.1.10 introduced deeply nested conditional types inside
 * `ActorConn` that exceed TypeScript's instantiation depth limit when
 * wrapped in `Omit`. This permissive index signature preserves the
 * "call any actor action on the object" DX while avoiding TS2589.
 * All reactive state properties above remain fully typed.
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
	_ActorName extends keyof ExtractActorsFromRegistry<Registry> &
		string = keyof ExtractActorsFromRegistry<Registry> & string,
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
	_ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
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

function proxyWithConnection<T extends object>(
	inner: T,
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	getConnection: () => ActorConn<any> | null,
	interceptAction?: ActionInterceptor,
): T {
	const methodCache = new WeakMap<object, Map<string, unknown>>();

	return new Proxy(inner, {
		get(target, prop, receiver) {
			if (Reflect.has(target, prop)) {
				return Reflect.get(target, prop, receiver);
			}
			const conn = getConnection();
			if (conn && typeof prop === "string") {
				const val = conn[prop as keyof typeof conn];
				if (typeof val === "function") {
					let connMethods = methodCache.get(conn);
					if (!connMethods) {
						connMethods = new Map<string, unknown>();
						methodCache.set(conn, connMethods);
					}

					const cached = connMethods.get(prop);
					if (cached) return cached;

					// When an interceptor is configured, wrap the call through it
					// so action middleware (timeout, error capture, loading tracking)
					// applies automatically to every proxied action call.
					const bound = interceptAction
						? // eslint-disable-next-line @typescript-eslint/no-explicit-any
							(...args: any[]) =>
								interceptAction(prop, args, () =>
									// biome-ignore lint/complexity/noBannedTypes: val is guaranteed to be a function here since it's proxied from the connection methods
									(val as Function).apply(conn, args),
								)
						: // eslint-disable-next-line @typescript-eslint/no-explicit-any
							(...args: any[]) =>
								// biome-ignore lint/complexity/noBannedTypes: val is guaranteed to be a function here since it's proxied from the connection methods
								(val as Function).apply(conn, args);
					connMethods.set(prop, bound);
					return bound;
				}

				return val;
			}

			// When an interceptor is configured and the connection is null,
			// return a function that routes through the interceptor so the
			// connection guard can fire and capture the error reactively.
			// Without this, calling actor.someAction() when disconnected
			// would throw TypeError: undefined is not a function.
			if (interceptAction && !conn && typeof prop === "string") {
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				return (...args: any[]) =>
					interceptAction(prop, args, () =>
						Promise.reject(
							new Error(
								`Action "${prop}" called while disconnected`,
							),
						),
					);
			}

			return undefined;
		},
	});
}

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
		opts: MaybeGetter<ActorOptions<Registry, ActorName>>,
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
		opts: ActorOptions<Registry, ActorName>,
	) => ReactiveActorHandle<Registry, ActorName>;

	/**
	 * Wake an actor without establishing a WebSocket connection.
	 *
	 * Sends a single HTTP `PUT /actors` (getOrCreate + resolve) to ensure the
	 * actor instance exists and is running. Useful for preloading actors on
	 * hover to eliminate cold-start latency on subsequent connections — similar
	 * to SvelteKit's `data-sveltekit-preload-data` pattern.
	 *
	 * - **No WebSocket** — only an HTTP resolve call, no persistent connection.
	 * - **Deduplicates** — same actor (name + key) is only resolved once per
	 *   RivetKit instance. Failed attempts are removed from the dedup set so
	 *   retries work.
	 * - **Fire-and-forget** — errors are silently caught. Preload failure
	 *   should never affect user experience.
	 *
	 * @example
	 * ```typescript
	 * // Preload a document actor on hover
	 * rivet.preloadActor({ name: 'document', key: ['doc', docId] });
	 * ```
	 *
	 * @param opts - Actor name and key to preload.
	 */
	preloadActor: <
		ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
	>(
		opts: PreloadActorOptions<Registry, ActorName>,
	) => void;
}

// ---------------------------------------------------------------------------
// Factory functions
// ---------------------------------------------------------------------------

/**
 * Options for creating a RivetKit instance, extending framework-base options
 * with Svelte-specific action middleware defaults.
 */
export interface SvelteRivetKitOptions<Registry extends AnyActorRegistry>
	extends CreateRivetKitOptions<Registry> {
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
}

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
) {
	// Internal implementations erase the ActorName generic. The deeply nested
	// conditional types inside ActorConn (rivetkit 2.1.10) exceed TypeScript's
	// instantiation depth when evaluated in generic function bodies. The public
	// RivetKit<Registry> interface provides full type safety to consumers.
	const { actionDefaults: clientActionDefaults, ...frameworkOpts } = opts;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	const { getOrCreateActor } = createVanillaRivetKit<Registry>(
		client,
		frameworkOpts,
	) as {
		getOrCreateActor: (actorOpts: any) => {
			mount: () => () => void;
			state: any;
		};
	};

	// -------------------------------------------------------------------
	// Action interceptor builder — creates a closure-based interceptor
	// that captures $state variables for reactive action tracking.
	//
	// The interceptor is called by proxyWithConnection for every forwarded
	// action call, providing: timeout, error capture to $state, loading
	// tracking, and lifecycle callbacks — without manual wrapping.
	// -------------------------------------------------------------------

	function buildInterceptor(
		defaults: ActionDefaults,
		getConn: () => ActorConn<unknown> | null,
		state: {
			getIsMutating: () => boolean;
			setIsMutating: (v: boolean) => void;
			getPendingActions: () => number;
			setPendingActions: (v: number) => void;
			setLastActionError: (v: Error | null) => void;
			setLastAction: (v: string | null) => void;
		},
	): ActionInterceptor {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		return (actionName: string, args: any[], call: () => Promise<any>) => {
			// Connection guard — fail fast if disconnected (default: enabled)
			if (defaults.guardConnection !== false) {
				const conn = getConn();
				if (!conn) {
					const err = new Error(
						`Action "${actionName}" called while disconnected`,
					);
					state.setLastActionError(err);
					state.setLastAction(actionName);
					defaults.onActionError?.(err, actionName);
					defaults.onActionSettled?.(actionName);

					const shouldThrow =
						typeof defaults.throwOnError === "function"
							? defaults.throwOnError(err, actionName)
							: (defaults.throwOnError ?? false);

					if (shouldThrow) return Promise.reject(err);
					return Promise.resolve(undefined);
				}
			}

			// Track pending actions
			const pending = state.getPendingActions() + 1;
			state.setPendingActions(pending);
			state.setIsMutating(true);
			state.setLastAction(actionName);
			defaults.onActionStart?.(actionName, args);

			// Execute with optional timeout race
			let timeoutId: ReturnType<typeof setTimeout> | undefined;
			const callPromise = call();

			// Suppress unhandled rejection if the timeout wins — the action
			// promise may reject after the catch block exits.
			callPromise.catch(() => {});

			const raced = defaults.timeout
				? Promise.race([
						callPromise,
						new Promise<never>((_, reject) => {
							timeoutId = setTimeout(
								() =>
									reject(
										new Error(
											`Action "${actionName}" timed out after ${defaults.timeout}ms`,
										),
									),
								defaults.timeout,
							);
						}),
					])
				: callPromise;

			return raced
				.then((result) => {
					state.setLastActionError(null);
					defaults.onActionSuccess?.(actionName, result);
					return result;
				})
				.catch((error) => {
					const err =
						error instanceof Error
							? error
							: new Error(String(error));
					state.setLastActionError(err);
					defaults.onActionError?.(err, actionName);

					// throwOnError: false (default) → resolve to undefined
					// throwOnError: true → re-throw
					// throwOnError: fn → call per-error
					const shouldThrow =
						typeof defaults.throwOnError === "function"
							? defaults.throwOnError(err, actionName)
							: (defaults.throwOnError ?? false);

					if (shouldThrow) throw err;
					return undefined;
				})
				.finally(() => {
					clearTimeout(timeoutId);
					const newPending = state.getPendingActions() - 1;
					state.setPendingActions(newPending);
					state.setIsMutating(newPending > 0);
					defaults.onActionSettled?.(actionName);
				});
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
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		let _connection = $state<ActorConn<any> | null>(null);
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		let _handle = $state<ActorHandle<any> | null>(null);
		let _connStatus = $state<ActorConnStatus>("idle" as ActorConnStatus);
		let _error = $state<Error | null>(null);
		let _lastError = $state<Error | null>(null);
		let _hasEverConnected = $state(false);
		let _hash = $state("");

		// Action tracking state (active when actionDefaults is configured)
		let _isMutating = $state(false);
		let _pendingActions = $state(0);
		let _lastActionError = $state<Error | null>(null);
		let _lastAction = $state<string | null>(null);

		// Resolve action defaults from the initial options (not reactive —
		// actionDefaults are structural config, not per-render state).
		const initialOpts = extract(optsOrGetter);
		const resolvedDefaults = mergeActionDefaults(
			clientActionDefaults,
			initialOpts?.actionDefaults,
		);

		const interceptAction = resolvedDefaults
			? buildInterceptor(resolvedDefaults, () => _connection, {
					getIsMutating: () => _isMutating,
					setIsMutating: (v) => {
						_isMutating = v;
					},
					getPendingActions: () => _pendingActions,
					setPendingActions: (v) => {
						_pendingActions = v;
					},
					setLastActionError: (v) => {
						_lastActionError = v;
					},
					setLastAction: (v) => {
						_lastAction = v;
					},
				})
			: undefined;

		$effect(() => {
			const actorOpts = extract(optsOrGetter);

			// Strip actionDefaults before passing to framework-base
			// (it doesn't know about our Svelte-specific extension)
			// eslint-disable-next-line @typescript-eslint/no-unused-vars
			const { actionDefaults: _ad, ...baseOpts } = actorOpts ?? {};

			const { mount, state: derived } = getOrCreateActor(baseOpts);
			const unmount = mount();

			const initial = derived.state;
			if (initial) {
				_connection = initial.connection;
				_handle = initial.handle;
				_connStatus = initial.connStatus;
				_error = initial.error;
				_lastError = initial.error ?? _lastError;
				_hasEverConnected =
					initial.connStatus === "connected" || _hasEverConnected;
				_hash = initial.hash ?? "";
			}

			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const unsub = derived.subscribe(
				({ currentVal }: { currentVal: any }) => {
					if (!currentVal) return;
					_connection = currentVal.connection;
					_handle = currentVal.handle;
					_connStatus = currentVal.connStatus;
					_error = currentVal.error;
					_lastError = currentVal.error ?? _lastError;
					_hasEverConnected =
						currentVal.connStatus === "connected" ||
						_hasEverConnected;
					_hash = currentVal.hash ?? "";
				},
			);

			return () => {
				unsub();
				unmount();
			};
		});

		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		function onEvent(
			eventName: string,
			handler: (...args: any[]) => void,
		): void {
			$effect(() => {
				if (!_connection) return;
				return _connection.on(eventName, handler);
			});
		}

		const inner = {
			get connection() {
				return _connection;
			},
			get handle() {
				return _handle;
			},
			get connStatus() {
				return _connStatus;
			},
			get error() {
				return _error;
			},
			get lastError() {
				return _lastError;
			},
			get isConnected() {
				return _connStatus === "connected";
			},
			get hasEverConnected() {
				return _hasEverConnected;
			},
			get hash() {
				return _hash;
			},
			// Action tracking
			get isMutating() {
				return _isMutating;
			},
			get pendingActions() {
				return _pendingActions;
			},
			get lastActionError() {
				return _lastActionError;
			},
			get lastAction() {
				return _lastAction;
			},
			resetActionState() {
				_lastActionError = null;
				_lastAction = null;
			},
			onEvent,
		};

		return proxyWithConnection(inner, () => _connection, interceptAction);
	}

	// -------------------------------------------------------------------
	// preloadActor — fire-and-forget actor warm-up via resolve()
	// -------------------------------------------------------------------

	/**
	 * Set of actor hashes that have already been preloaded (or are in-flight).
	 * Keyed by `"actorName:key0:key1:..."` so the same actor instance is only
	 * resolved once per RivetKit lifetime.
	 */
	const _preloaded = new Set<string>();

	function preloadActor(opts: PreloadActorOptions): void {
		if (!BROWSER) return;

		const keyArray = Array.isArray(opts.key) ? opts.key : [opts.key];
		const hash = `${String(opts.name)}:${keyArray.join(":")}`;
		if (_preloaded.has(hash)) return;
		_preloaded.add(hash);

		const accessor = (client as Record<string, unknown>)[
			opts.name as string
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
		] as any;

		const handle = accessor.getOrCreate(
			keyArray,
			opts.createWithInput != null
				? { createWithInput: opts.createWithInput }
				: undefined,
		);

		handle.resolve().catch(() => {
			_preloaded.delete(hash);
		});
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
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		let _connection = $state<ActorConn<any> | null>(null);
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		let _handle = $state<ActorHandle<any> | null>(null);
		let _connStatus = $state<ActorConnStatus>("idle" as ActorConnStatus);
		let _error = $state<Error | null>(null);
		let _lastError = $state<Error | null>(null);
		let _hasEverConnected = $state(false);
		let _hash = $state("");

		// Action tracking state (active when actionDefaults is configured)
		let _isMutating = $state(false);
		let _pendingActions = $state(0);
		let _lastActionError = $state<Error | null>(null);
		let _lastAction = $state<string | null>(null);

		// Resolve action defaults: actor-level overrides client-level
		const resolvedDefaults = mergeActionDefaults(
			clientActionDefaults,
			actorOpts?.actionDefaults,
		);

		const interceptAction = resolvedDefaults
			? buildInterceptor(resolvedDefaults, () => _connection, {
					getIsMutating: () => _isMutating,
					setIsMutating: (v) => {
						_isMutating = v;
					},
					getPendingActions: () => _pendingActions,
					setPendingActions: (v) => {
						_pendingActions = v;
					},
					setLastActionError: (v) => {
						_lastActionError = v;
					},
					setLastAction: (v) => {
						_lastAction = v;
					},
				})
			: undefined;

		// Strip actionDefaults before passing to framework-base
		// eslint-disable-next-line @typescript-eslint/no-unused-vars
		const { actionDefaults: _ad, ...baseOpts } = actorOpts ?? {};

		const { mount, state: derived } = getOrCreateActor(baseOpts);

		const _eventListeners = new Set<{
			event: string;
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			handler: (...args: any[]) => void;
			unsubscribe?: () => void;
		}>();

		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		function _applyState(val: any): void {
			if (!val) return;
			const prevConn = _connection;

			_connection = val.connection;
			_handle = val.handle;
			_connStatus = val.connStatus;
			_error = val.error;
			_lastError = val.error ?? _lastError;
			_hasEverConnected =
				val.connStatus === "connected" || _hasEverConnected;
			_hash = val.hash ?? "";

			if (prevConn !== _connection) {
				for (const listener of _eventListeners) {
					if (listener.unsubscribe) listener.unsubscribe();
					if (_connection) {
						listener.unsubscribe = _connection.on(
							listener.event,
							listener.handler,
						);
					}
				}
			}
		}

		_applyState(derived.state);
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const _unsub = derived.subscribe(
			({ currentVal }: { currentVal: any }) => _applyState(currentVal),
		);

		const inner = {
			get connection() {
				return _connection;
			},
			get handle() {
				return _handle;
			},
			get connStatus() {
				return _connStatus;
			},
			get error() {
				return _error;
			},
			get lastError() {
				return _lastError;
			},
			get isConnected() {
				return _connStatus === "connected";
			},
			get hasEverConnected() {
				return _hasEverConnected;
			},
			get hash() {
				return _hash;
			},
			// Action tracking
			get isMutating() {
				return _isMutating;
			},
			get pendingActions() {
				return _pendingActions;
			},
			get lastActionError() {
				return _lastActionError;
			},
			get lastAction() {
				return _lastAction;
			},
			resetActionState() {
				_lastActionError = null;
				_lastAction = null;
			},

			mount() {
				return mount();
			},

			dispose() {
				_unsub();
				for (const listener of _eventListeners) {
					if (listener.unsubscribe) listener.unsubscribe();
				}
				_eventListeners.clear();
			},

			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			onEvent(
				eventName: string,
				handler: (...args: any[]) => void,
			): () => void {
				const listener: {
					event: string;
					// eslint-disable-next-line @typescript-eslint/no-explicit-any
					handler: (...args: any[]) => void;
					unsubscribe?: () => void;
				} = { event: eventName, handler };

				if (_connection) {
					listener.unsubscribe = _connection.on(eventName, handler);
				}
				_eventListeners.add(listener);

				return () => {
					if (listener.unsubscribe) listener.unsubscribe();
					_eventListeners.delete(listener);
				};
			},
		};

		return proxyWithConnection(inner, () => _connection, interceptAction);
	}

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	return { useActor, createReactiveActor, preloadActor } as any;
}
