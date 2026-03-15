/**
 * Sandbox actor — wraps a sandbox-agent as a RivetKit actor.
 *
 * ## Lifecycle
 *
 * The sandbox actor manages a remote sandbox environment (Docker container,
 * E2B sandbox, Daytona workspace, or custom provider) and exposes every
 * sandbox-agent SDK method as an actor action.
 *
 * **Creation:** On the first action call, `ensureAgent` lazily provisions
 * the sandbox via `provider.create()` and connects to it via
 * `provider.connectAgent()`. The sandbox ID and provider name are persisted
 * in state so subsequent wake cycles reconnect to the same sandbox.
 *
 * **Sleep/Wake:** When the actor sleeps, `onSleep` tears down the live
 * agent WebSocket connection and clears all in-memory subscriptions and
 * timers. Vars are ephemeral and recreated fresh on each wake cycle via
 * `createVars`. On the next action call after wake, `ensureAgent` reconnects
 * to the existing sandbox (identified by `state.sandboxId`) and re-subscribes
 * to all sessions listed in `state.subscribedSessionIds`. If the provider
 * supports `wake()`, it is called before connecting to ensure the sandbox
 * process is running (e.g. Daytona sandboxes may sleep and stop the
 * sandbox-agent process).
 *
 * **Destroy:** `onDestroy` tears down the connection, then calls
 * `provider.destroy()` to delete the sandbox environment. The custom
 * `destroy` action allows users to destroy the sandbox without destroying
 * the actor, setting `state.sandboxDestroyed = true`. After this, proxy
 * actions that require a live sandbox throw, but read-only actions
 * (`listSessions`, `getSession`, `getEvents`) fall back to the local
 * SQLite persistence layer so transcripts remain accessible.
 *
 * ## Session management
 *
 * Session subscriptions and turn tracking are handled by `./session.ts`.
 * The actor prevents sleep while any session has an active prompt turn
 * or while user-provided hooks are executing. Idle timers warn and
 * eventually force-clear stale turns to prevent the actor from staying
 * awake indefinitely.
 *
 * ## Prevent-sleep coordination
 *
 * `setPreventSleep(true)` is set whenever:
 * - A session has an active prompt turn (tracked via JSON-RPC id matching)
 * - A user hook (onSessionEvent, onPermissionRequest) is executing
 *
 * It is cleared when all active turns complete and all hooks finish.
 * To avoid a race between sending a prompt and receiving the first event,
 * session-creating and message-sending actions immediately mark the session
 * active with idle timers.
 */

import type { DatabaseProvider } from "@/actor/database";
import { actor } from "@/actor/mod";
import type { RawAccess } from "@/db/config";
import { db } from "@/db/mod";
import type { SandboxAgent } from "sandbox-agent";
import {
	type SandboxActorConfig,
	type SandboxActorConfigInput,
	type SandboxActorOptionsRuntime,
	SandboxActorConfigSchema,
} from "../config";
import { SqliteSessionPersistDriver } from "../session-persist-driver";
import {
	type SandboxActionContext,
	type SandboxActorActions,
	type SandboxActorProvider,
	type SandboxActorState,
	type SandboxActorVars,
	SANDBOX_AGENT_ACTION_METHODS,
} from "../types";
import { migrateSandboxTables } from "./db";
import {
	addSubscribedSession,
	clearAllActiveSessions,
	clearAllSessionTimers,
	markSessionActiveInMemory,
	removeSubscribedSession,
	subscribeToSession,
	syncPreventSleep,
} from "./session";

// --- Proxy action type definitions ---

type SandboxProxyActionDefinitions<TConnParams> = {
	[K in keyof SandboxActorActions]: (
		c: SandboxActionContext<TConnParams>,
		...args: Parameters<SandboxActorActions[K]>
	) => ReturnType<SandboxActorActions[K]>;
};

// --- Agent runtime lifecycle ---

/**
 * Tears down the live sandbox-agent connection and all associated
 * in-memory state (event subscriptions, timers, hook tracking).
 * Does NOT destroy the sandbox itself.
 */
async function teardownAgentRuntime(
	vars: SandboxActorVars,
): Promise<void> {
	for (const subscription of vars.unsubscribeBySessionId.values()) {
		subscription.event?.();
		subscription.permission?.();
	}
	vars.unsubscribeBySessionId.clear();
	clearAllSessionTimers(vars);
	vars.activeHooks.clear();

	if (vars.sandboxAgentClient) {
		try {
			await vars.sandboxAgentClient.dispose();
		} finally {
			vars.sandboxAgentClient = null;
		}
	}

	vars.provider = null;
}

/**
 * Lazily provisions and connects to the sandbox. On the first call, this
 * creates the sandbox via the provider. On subsequent calls (e.g. after
 * wake), it reconnects to the existing sandbox. Short-circuits if the
 * agent client is already connected.
 *
 * Steps:
 * 1. Resolve the provider (static or via createProvider callback)
 * 2. Create the sandbox if no sandboxId exists yet
 * 3. If the provider supports `wake()`, call it to ensure the sandbox
 *    process is running
 * 4. Connect the sandbox-agent client with SQLite persistence
 * 5. Re-subscribe to all persisted session IDs
 */
async function ensureAgent<TConnParams>(
	c: SandboxActionContext<TConnParams>,
	config: SandboxActorConfig<TConnParams>,
	persistRawEvents: boolean,
): Promise<SandboxAgent> {
	if (c.vars.sandboxAgentClient) {
		return c.vars.sandboxAgentClient;
	}

	// Resolve the provider, either from the static config or by calling
	// the factory function. Cache it in vars for the rest of this wake cycle.
	let provider: SandboxActorProvider;
	if (c.vars.provider) {
		provider = c.vars.provider;
	} else {
		provider =
			config.provider !== undefined
				? config.provider
				: await config.createProvider(c);

		if (c.state.providerName && c.state.providerName !== provider.name) {
			throw new Error(
				`sandbox actor provider mismatch: expected ${c.state.providerName}, received ${provider.name}`,
			);
		}

		if (!c.state.providerName) {
			c.state.providerName = provider.name;
		}

		c.vars.provider = provider;
	}

	// Create the sandbox if this is the first time.
	if (!c.state.sandboxId) {
		c.state.sandboxId = await provider.create({
			actorId: c.actorId,
			actorKey: [...c.key],
		});
	}

	// Some providers (e.g. Daytona) need to restart the sandbox-agent
	// process after the sandbox sleeps or restarts.
	if (provider.wake) {
		await provider.wake(c.state.sandboxId);
	}

	// Restart idle timers for any sessions that were active before sleep.
	const options = config.options as SandboxActorOptionsRuntime;
	for (const sessionId of c.vars.activeSessionIds) {
		markSessionActiveInMemory(c, options, sessionId);
		syncPreventSleep(c);
	}

	// Connect to the sandbox-agent with SQLite-backed persistence.
	c.vars.sandboxAgentClient = await provider.connectAgent(c.state.sandboxId, {
		persist: new SqliteSessionPersistDriver(c.db, persistRawEvents),
		waitForHealth: true,
	});

	// Re-subscribe to all sessions that were active before sleep.
	for (const sessionId of c.state.subscribedSessionIds) {
		subscribeToSession(c, config, sessionId);
	}

	return c.vars.sandboxAgentClient;
}

// --- Read-only fallback actions ---

// These actions can read from the local SQLite persistence layer even after
// the sandbox has been destroyed, allowing transcript access.
const READ_ONLY_ACTIONS = new Set([
	"listSessions",
	"getSession",
	"getEvents",
]);

// --- Session-returning action detection ---

// Actions that return a session object. After these actions, the actor
// auto-subscribes to the returned session's event stream.
const SESSION_RETURNING_ACTIONS = new Set([
	"createSession",
	"resumeSession",
	"resumeOrCreateSession",
	"getSession",
]);

// Actions that send messages to a session. These immediately mark the
// session active to prevent a race between sending and receiving the
// first event.
const SESSION_SENDING_ACTIONS = new Set([
	"rawSendSessionMethod",
	"respondPermission",
	"rawRespondPermission",
]);

function isSessionLike(value: unknown): value is { id: string } {
	return (
		typeof value === "object" &&
		value !== null &&
		"id" in value &&
		typeof (value as Record<string, unknown>).id === "string"
	);
}

// --- Proxy action builder ---

/**
 * Generates an action for each sandbox-agent SDK method. Each proxy action:
 *
 * 1. Calls `ensureAgent()` to lazily connect to the sandbox
 * 2. Forwards the call to the corresponding `SandboxAgent` method
 * 3. Handles post-action side effects:
 *    - `dispose`: tears down the agent runtime
 *    - `destroySession`: unsubscribes from the destroyed session
 *    - Session-returning actions: auto-subscribes to the session
 *    - `listSessions`: auto-subscribes to all returned sessions
 *    - Session-sending actions: immediately marks the session active
 *      to prevent sleep before the first event arrives
 */
function buildProxyActions<TConnParams>(
	config: SandboxActorConfig<TConnParams>,
): SandboxProxyActionDefinitions<TConnParams> {
	const actions = {} as Record<
		string,
		(c: SandboxActionContext<TConnParams>, ...args: unknown[]) => Promise<unknown>
	>;

	for (const actionName of SANDBOX_AGENT_ACTION_METHODS) {
		actions[actionName] = async (
			c: SandboxActionContext<TConnParams>,
			...args: unknown[]
		) => {
			// After sandbox destruction, only read-only actions are allowed.
			// These fall back to the SQLite persistence layer.
			if (c.state.sandboxDestroyed) {
				if (READ_ONLY_ACTIONS.has(actionName)) {
					const persist = new SqliteSessionPersistDriver(
						c.db,
						config.persistRawEvents ?? false,
					);
					if (actionName === "listSessions") {
						return persist.listSessions(args[0] as any);
					}
					if (actionName === "getSession") {
						return persist.getSession(args[0] as string);
					}
					if (actionName === "getEvents") {
						return persist.listEvents(args[0] as any);
					}
				}
				throw new Error(
					"sandbox has been destroyed; only read-only actions (listSessions, getSession, getEvents) are available",
				);
			}

			const options = config.options as SandboxActorOptionsRuntime;

			// For session-sending actions, immediately mark the session
			// active before dispatching to prevent the actor from sleeping
			// between sending the message and receiving the first event.
			if (
				SESSION_SENDING_ACTIONS.has(actionName) &&
				typeof args[0] === "string"
			) {
				markSessionActiveInMemory(c, options, args[0]);
				syncPreventSleep(c);
			}

			// Connect to the sandbox-agent and forward the method call.
			const agent = await ensureAgent(
				c,
				config,
				config.persistRawEvents ?? false,
			);
			const method = agent[actionName] as (
				...innerArgs: unknown[]
			) => unknown;
			const result = await method.apply(agent, args);

			// Post-action side effects: manage session subscriptions based
			// on what the action returned.
			if (actionName === "dispose") {
				await teardownAgentRuntime(c.vars);
				clearAllActiveSessions(c);
			} else if (
				actionName === "destroySession" &&
				isSessionLike(result)
			) {
				const sub = c.vars.unsubscribeBySessionId.get(result.id);
				sub?.event?.();
				sub?.permission?.();
				c.vars.unsubscribeBySessionId.delete(result.id);
				removeSubscribedSession(c, result.id);
			} else if (
				SESSION_RETURNING_ACTIONS.has(actionName) &&
				isSessionLike(result)
			) {
				addSubscribedSession(c, result.id);
				subscribeToSession(c, config, result.id);
			} else if (
				actionName === "listSessions" &&
				result &&
				typeof result === "object"
			) {
				const items = (result as { items?: unknown }).items;
				if (Array.isArray(items)) {
					for (const item of items) {
						if (isSessionLike(item)) {
							addSubscribedSession(c, item.id);
							subscribeToSession(c, config, item.id);
						}
					}
				}
			}

			return result;
		};
	}

	return actions as unknown as SandboxProxyActionDefinitions<TConnParams>;
}

// --- Public API ---

export function sandboxActor<TConnParams = undefined>(
	config: SandboxActorConfigInput<TConnParams>,
) {
	const parsedConfig = SandboxActorConfigSchema.parse(
		config,
	) as SandboxActorConfig<TConnParams> & {
		options: SandboxActorOptionsRuntime;
	};

	return actor<
		SandboxActorState,
		TConnParams,
		undefined,
		SandboxActorVars,
		undefined,
		DatabaseProvider<RawAccess>,
		Record<never, never>,
		Record<never, never>
	>({
		createState: async () => ({
			sandboxId: null,
			providerName: null,
			subscribedSessionIds: [],
			sandboxDestroyed: false,
		}),
		createVars: () => ({
			sandboxAgentClient: null,
			provider: null,
			activeSessionIds: new Set<string>(),
			activePromptRequestIdsBySessionId: new Map<string, string[]>(),
			lastEventAtBySessionId: new Map<string, number>(),
			unsubscribeBySessionId: new Map(),
			activeHooks: new Set<Promise<void>>(),
			warningTimeoutBySessionId: new Map(),
			staleTimeoutBySessionId: new Map(),
		}),
		db: db({
			onMigrate: migrateSandboxTables,
		}),
		onSleep: async (c) => {
			await teardownAgentRuntime(c.vars);
		},
		onDestroy: async (c) => {
			const sandboxContext =
				c as SandboxActionContext<TConnParams>;
			clearAllActiveSessions(sandboxContext);
			await teardownAgentRuntime(sandboxContext.vars);

			if (sandboxContext.state.sandboxId) {
				try {
					// Resolve the provider to call destroy on the sandbox.
					let provider: SandboxActorProvider | null =
						sandboxContext.vars.provider;
					if (!provider) {
						provider =
							parsedConfig.provider !== undefined
								? parsedConfig.provider
								: await parsedConfig.createProvider(
										sandboxContext,
									);
					}
					await provider.destroy(sandboxContext.state.sandboxId);
				} finally {
					sandboxContext.state.sandboxId = null;
					sandboxContext.state.providerName = null;
				}
			}

			sandboxContext.state.subscribedSessionIds = [];
		},
		onBeforeConnect: parsedConfig.onBeforeConnect,
		actions: {
			// Destroys the sandbox environment but keeps the actor alive so
			// session transcripts remain accessible via read-only actions.
			// If `destroyActor` is set in the config, the actor is also
			// destroyed after the sandbox.
			destroy: async (c: SandboxActionContext<TConnParams>) => {
				if (c.state.sandboxDestroyed) {
					return;
				}

				clearAllActiveSessions(c);
				await teardownAgentRuntime(c.vars);

				if (c.state.sandboxId) {
					let provider: SandboxActorProvider | null =
						c.vars.provider;
					if (!provider) {
						provider =
							parsedConfig.provider !== undefined
								? parsedConfig.provider
								: await parsedConfig.createProvider(c);
					}
					await provider.destroy(c.state.sandboxId);
					c.state.sandboxId = null;
				}

				c.state.sandboxDestroyed = true;

				if (parsedConfig.destroyActor) {
					c.destroy();
				}
			},
			...buildProxyActions(parsedConfig),
		},
	});
}
