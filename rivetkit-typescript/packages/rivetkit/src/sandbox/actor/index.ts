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
 * the sandbox via `SandboxAgent.start()`, which calls `provider.create()`
 * and connects to the running sandbox-agent server. The sandbox ID and
 * provider name are persisted in state so subsequent wake cycles reconnect
 * to the same sandbox.
 *
 * **Sleep/Wake:** When the actor sleeps, `onSleep` tears down the live
 * agent WebSocket connection and clears all in-memory subscriptions and
 * timers. Vars are ephemeral and recreated fresh on each wake cycle via
 * `createVars`. On the next action call after wake, `ensureAgent` reconnects
 * to the existing sandbox (identified by `state.sandboxId`) via
 * `SandboxAgent.start()` with the persisted sandbox ID and re-subscribes
 * to all sessions listed in `state.subscribedSessionIds`. The upstream
 * provider's `ensureServer()` hook (if implemented) is called automatically
 * by `SandboxAgent.start()` to ensure the sandbox-agent process is running.
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

import type { DatabaseProvider } from "@/common/database/config";
import { actor } from "@/actor/mod";
import type { RawAccess } from "@/common/database/config";
import { db } from "@/common/database/mod";
import { SandboxAgent, type SandboxProvider } from "sandbox-agent";
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
async function teardownAgentRuntime(vars: SandboxActorVars): Promise<void> {
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

async function resolveProvider<TConnParams>(
	c: SandboxActionContext<TConnParams>,
	config: SandboxActorConfig<TConnParams>,
): Promise<SandboxProvider> {
	if (c.vars.provider) {
		return c.vars.provider;
	}

	const provider =
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
	return provider;
}

/**
 * Lazily provisions and connects to the sandbox. On the first call, this
 * creates the sandbox via the provider. On subsequent calls (e.g. after
 * wake), it reconnects to the existing sandbox. Short-circuits if the
 * agent client is already connected.
 *
 * Steps:
 * 1. Resolve the provider (static or via createProvider callback)
 * 2. Call `SandboxAgent.start()` which handles:
 *    - Creating the sandbox if no sandboxId is provided
 *    - Calling `ensureServer()` if the provider implements it
 *    - Connecting to the sandbox-agent server
 * 3. Persist the sandbox ID from the started client
 * 4. Re-subscribe to all persisted session IDs
 */
async function ensureAgent<TConnParams>(
	c: SandboxActionContext<TConnParams>,
	config: SandboxActorConfig<TConnParams>,
	persistRawEvents: boolean,
): Promise<SandboxAgent> {
	if (c.vars.sandboxAgentClient) {
		return c.vars.sandboxAgentClient;
	}

	const provider = await resolveProvider(c, config);

	c.vars.sandboxAgentClient = await SandboxAgent.start({
		sandbox: provider,
		sandboxId: c.state.sandboxId ?? undefined,
		persist: new SqliteSessionPersistDriver(c.db, persistRawEvents),
	});

	// Persist the sandbox ID so future wake cycles reconnect to the same sandbox.
	if (!c.state.sandboxId && c.vars.sandboxAgentClient.sandboxId) {
		c.state.sandboxId = c.vars.sandboxAgentClient.sandboxId;
	}

	// Re-subscribe to all sessions that were active before sleep.
	for (const sessionId of c.state.subscribedSessionIds) {
		subscribeToSession(c, config, sessionId);
	}

	return c.vars.sandboxAgentClient;
}

// --- Read-only fallback actions ---

// These actions can read from the local SQLite persistence layer even after
// the sandbox has been destroyed, allowing transcript access.
const READ_ONLY_ACTIONS = new Set(["listSessions", "getSession", "getEvents"]);

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

/**
 * HACK: Convert Session instances to plain SessionRecords for RPC
 * serialization. The root cause is that sandbox-agent SDK methods return
 * Session class instances (which hold internal references like the
 * SandboxAgent) instead of plain SessionRecord objects. The proper fix is
 * to have sandbox-agent return SessionRecords directly from its public API
 * so callers don't need to post-process results.
 */
function toSerializable(value: unknown): unknown {
	if (value === null || value === undefined) return value;
	if (typeof value !== "object") return value;

	// Session instance: convert via toRecord().
	if (
		"toRecord" in value &&
		typeof (value as Record<string, unknown>).toRecord === "function"
	) {
		return (value as { toRecord(): unknown }).toRecord();
	}

	// Array: recurse into elements.
	if (Array.isArray(value)) {
		return value.map(toSerializable);
	}

	// Plain object: recurse into values. Only process POJOs to avoid
	// serializing class instances with non-data properties.
	const proto = Object.getPrototypeOf(value);
	if (proto === Object.prototype || proto === null) {
		const out: Record<string, unknown> = {};
		for (const [k, v] of Object.entries(value)) {
			out[k] = toSerializable(v);
		}
		return out;
	}

	return value;
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
		(
			c: SandboxActionContext<TConnParams>,
			...args: unknown[]
		) => Promise<unknown>
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

			return toSerializable(result);
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
		options: {
			// Sandbox operations (container startup, agent install, session
			// creation) are inherently slower than normal actor actions.
			actionTimeout: 120_000,
		},
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
			const sandboxContext = c as SandboxActionContext<TConnParams>;
			clearAllActiveSessions(sandboxContext);
			await teardownAgentRuntime(sandboxContext.vars);

			if (sandboxContext.state.sandboxId) {
				try {
					const provider = await resolveProvider(
						sandboxContext,
						parsedConfig,
					);
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
					const provider = await resolveProvider(c, parsedConfig);
					await provider.destroy(c.state.sandboxId);
					c.state.sandboxId = null;
				}

				c.state.sandboxDestroyed = true;

				if (parsedConfig.destroyActor) {
					c.destroy();
				}
			},
			getSandboxUrl: async (c: SandboxActionContext<TConnParams>) => {
				if (c.state.sandboxDestroyed) {
					throw new Error("sandbox has been destroyed");
				}

				const provider = await resolveProvider(c, parsedConfig);

				// Ensure the sandbox exists so we have a sandbox ID.
				if (!c.state.sandboxId) {
					const agent = await ensureAgent(
						c,
						parsedConfig,
						parsedConfig.persistRawEvents ?? false,
					);
					if (!c.state.sandboxId && agent.sandboxId) {
						c.state.sandboxId = agent.sandboxId;
					}
				}

				if (!c.state.sandboxId) {
					throw new Error("sandbox ID is not available");
				}

				if (!provider.getUrl) {
					throw new Error(
						`provider "${provider.name}" does not support getUrl; direct sandbox URL access is not available for this provider`,
					);
				}

				return { url: await provider.getUrl(c.state.sandboxId) };
			},
			...buildProxyActions(parsedConfig),
		},
	});
}
