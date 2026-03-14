/**
 * Session lifecycle management for the sandbox actor.
 *
 * Manages three concerns:
 *
 * 1. **Subscription tracking** — which sandbox-agent sessions this actor is
 *    listening to for events and permission requests. Subscriptions are
 *    persisted in `state.subscribedSessionIds` so they survive sleep/wake.
 *
 * 2. **Active turn tracking** — detects when a session has an in-flight
 *    prompt turn by observing JSON-RPC envelopes on the event stream.
 *    A client `session/prompt` request starts a turn; the matching agent
 *    response (same JSON-RPC id) ends it. While any turn is active the
 *    actor prevents sleep.
 *
 * 3. **Idle timers** — if a turn appears stuck (no events for `warningAfterMs`),
 *    a warning is logged. After `staleAfterMs` the turn state is force-cleared
 *    so a missing terminal response cannot keep the actor awake forever.
 */

import type { SessionPermissionRequest } from "sandbox-agent";
import type { SandboxActorConfig, SandboxActorOptionsRuntime } from "../config";
import type {
	SandboxActionContext,
	SandboxActorVars,
	SandboxSessionEvent,
} from "../types";

// --- Prevent-sleep synchronization ---

export function syncPreventSleep<TConnParams>(
	c: SandboxActionContext<TConnParams>,
): void {
	c.setPreventSleep(
		c.vars.activeHooks.size > 0 || c.vars.activeSessionIds.size > 0,
	);
}

// --- Session timer management ---

function clearTimerMap(
	map: Map<string, ReturnType<typeof setTimeout>>,
	sessionId: string,
): void {
	const timeout = map.get(sessionId);
	if (timeout) {
		clearTimeout(timeout);
		map.delete(sessionId);
	}
}

function clearTimerMapAll(
	map: Map<string, ReturnType<typeof setTimeout>>,
): void {
	for (const timeout of map.values()) {
		clearTimeout(timeout);
	}
	map.clear();
}

function clearSessionTimers(vars: SandboxActorVars, sessionId: string): void {
	clearTimerMap(vars.warningTimeoutBySessionId, sessionId);
	clearTimerMap(vars.staleTimeoutBySessionId, sessionId);
}

export function clearAllSessionTimers(vars: SandboxActorVars): void {
	clearTimerMapAll(vars.warningTimeoutBySessionId);
	clearTimerMapAll(vars.staleTimeoutBySessionId);
}

// Schedules warning and stale timeouts for a session based on the last
// event timestamp. If the session goes idle for too long, the stale
// timeout clears the active turn so the actor can sleep.
function scheduleSessionTimers<TConnParams>(
	c: SandboxActionContext<TConnParams>,
	options: SandboxActorOptionsRuntime,
	sessionId: string,
): void {
	clearSessionTimers(c.vars, sessionId);

	const lastEventAt = c.vars.lastEventAtBySessionId.get(sessionId);
	if (lastEventAt === undefined) {
		return;
	}

	const warningDelay = Math.max(
		0,
		options.warningAfterMs - (Date.now() - lastEventAt),
	);
	c.vars.warningTimeoutBySessionId.set(
		sessionId,
		setTimeout(() => {
			if (!c.vars.activeSessionIds.has(sessionId)) {
				return;
			}

			c.log.warn({
				msg: "sandbox actor turn is still active without new session events",
				sessionId,
				idleMs: Date.now() - lastEventAt,
			});
		}, warningDelay),
	);

	const staleDelay = Math.max(
		0,
		options.staleAfterMs - (Date.now() - lastEventAt),
	);
	c.vars.staleTimeoutBySessionId.set(
		sessionId,
		setTimeout(() => {
			if (!c.vars.activeSessionIds.has(sessionId)) {
				return;
			}

			c.log.warn({
				msg: "sandbox actor cleared stale active turn state after inactivity timeout",
				sessionId,
				idleMs: Date.now() - lastEventAt,
			});
			clearSessionActiveInMemory(c, sessionId);
			syncPreventSleep(c);
		}, staleDelay),
	);
}

// --- Session active-state tracking (in-memory only) ---

export function markSessionActiveInMemory<TConnParams>(
	c: SandboxActionContext<TConnParams>,
	options: SandboxActorOptionsRuntime,
	sessionId: string,
	requestId?: string,
): void {
	c.vars.activeSessionIds.add(sessionId);
	if (requestId) {
		const requestIds =
			c.vars.activePromptRequestIdsBySessionId.get(sessionId) ?? [];
		if (!requestIds.includes(requestId)) {
			requestIds.push(requestId);
			c.vars.activePromptRequestIdsBySessionId.set(
				sessionId,
				requestIds,
			);
		}
	}

	c.vars.lastEventAtBySessionId.set(sessionId, Date.now());
	scheduleSessionTimers(c, options, sessionId);
}

function clearSessionActiveInMemory<TConnParams>(
	c: SandboxActionContext<TConnParams>,
	sessionId: string,
	requestId?: string,
): void {
	if (requestId) {
		const remaining =
			(c.vars.activePromptRequestIdsBySessionId.get(sessionId) ?? []).filter(
				(activeRequestId) => activeRequestId !== requestId,
			);
		if (remaining.length > 0) {
			c.vars.activePromptRequestIdsBySessionId.set(
				sessionId,
				remaining,
			);
			return;
		}
	}

	c.vars.activeSessionIds.delete(sessionId);
	c.vars.activePromptRequestIdsBySessionId.delete(sessionId);
	c.vars.lastEventAtBySessionId.delete(sessionId);
	clearSessionTimers(c.vars, sessionId);
}

// --- Session subscription management ---

export function addSubscribedSession<TConnParams>(
	c: SandboxActionContext<TConnParams>,
	sessionId: string,
): void {
	if (c.state.subscribedSessionIds.includes(sessionId)) {
		return;
	}
	c.state.subscribedSessionIds.push(sessionId);
}

export function removeSubscribedSession<TConnParams>(
	c: SandboxActionContext<TConnParams>,
	sessionId: string,
): void {
	clearSessionActiveInMemory(c, sessionId);
	c.state.subscribedSessionIds = c.state.subscribedSessionIds.filter(
		(id) => id !== sessionId,
	);
	syncPreventSleep(c);
}

export function clearAllActiveSessions<TConnParams>(
	c: SandboxActionContext<TConnParams>,
): void {
	c.vars.activeSessionIds.clear();
	c.vars.activePromptRequestIdsBySessionId.clear();
	c.vars.lastEventAtBySessionId.clear();
	clearAllSessionTimers(c.vars);
	syncPreventSleep(c);
}

// --- Hook execution ---

/**
 * Wraps a user-provided callback (onSessionEvent, onPermissionRequest) with
 * active-hook tracking and error isolation. The hook promise is added to
 * `vars.activeHooks` so prevent-sleep stays accurate, and removed on
 * completion. Errors are logged but do not crash the actor.
 */
function runHook<TConnParams>(
	c: SandboxActionContext<TConnParams>,
	sessionId: string,
	name: "onSessionEvent" | "onPermissionRequest",
	callback: () => void | Promise<void>,
): void {
	const promise = Promise.resolve(callback())
		.catch((error) => {
			c.log.error({
				msg: `sandbox actor ${name} hook failed`,
				sessionId,
				error,
			});
		})
		.finally(() => {
			c.vars.activeHooks.delete(promise);
			syncPreventSleep(c);
		});

	c.vars.activeHooks.add(promise);
	syncPreventSleep(c);

	c.waitUntil(promise);
}

// --- Turn tracking from session events ---

/**
 * Inspects raw JSON-RPC envelopes from the sandbox-agent event stream to
 * detect prompt turn boundaries. A client-side `session/prompt` request
 * marks the session active; the matching agent-side response (same
 * JSON-RPC id) clears it. Any intermediate event refreshes the idle timer.
 */
export function trackSessionTurnFromEvent<TConnParams>(
	c: SandboxActionContext<TConnParams>,
	options: SandboxActorOptionsRuntime,
	sessionId: string,
	event: SandboxSessionEvent,
): void {
	const payload = event.payload as Record<string, unknown> | null | undefined;
	const method =
		typeof payload?.method === "string" ? payload.method : null;
	const rawId = payload?.id;
	const id =
		typeof rawId === "string"
			? rawId
			: typeof rawId === "number"
				? String(rawId)
				: null;

	if (event.sender === "client" && method === "session/prompt") {
		markSessionActiveInMemory(
			c,
			options,
			sessionId,
			id ?? `session-prompt:${event.id}`,
		);
		syncPreventSleep(c);
		return;
	}

	if (!c.vars.activeSessionIds.has(sessionId)) {
		return;
	}

	if (event.sender === "agent" && id) {
		const requestIds =
			c.vars.activePromptRequestIdsBySessionId.get(sessionId) ?? [];
		if (requestIds.length === 0 || requestIds.includes(id)) {
			clearSessionActiveInMemory(c, sessionId, id);
			syncPreventSleep(c);
			return;
		}
	}

	// Any other event from an active session refreshes the idle timer.
	c.vars.lastEventAtBySessionId.set(sessionId, Date.now());
	scheduleSessionTimers(c, options, sessionId);
}

// --- Session event subscriptions ---

/**
 * Subscribes to a session's event and permission streams on the live
 * sandbox-agent connection. Tracks the unsubscribe callbacks so they
 * can be cleaned up on teardown.
 */
export function subscribeToSession<TConnParams>(
	c: SandboxActionContext<TConnParams>,
	config: SandboxActorConfig<TConnParams>,
	sessionId: string,
): void {
	if (c.vars.unsubscribeBySessionId.has(sessionId)) {
		return;
	}

	const client = c.vars.sandboxAgentClient;
	if (!client) {
		return;
	}

	const options = config.options as SandboxActorOptionsRuntime;

	const event = client.onSessionEvent(sessionId, (sessionEvent) => {
		trackSessionTurnFromEvent(c, options, sessionId, sessionEvent);

		if (!config.onSessionEvent) {
			return;
		}

		runHook(c, sessionId, "onSessionEvent", () =>
			config.onSessionEvent!(c, sessionId, sessionEvent),
		);
	});

	const permission = client.onPermissionRequest(
		sessionId,
		(request: SessionPermissionRequest) => {
			markSessionActiveInMemory(c, options, sessionId);
			syncPreventSleep(c);

			if (!config.onPermissionRequest) {
				return;
			}

			runHook(c, sessionId, "onPermissionRequest", () =>
				config.onPermissionRequest!(c, sessionId, request),
			);
		},
	);

	c.vars.unsubscribeBySessionId.set(sessionId, { event, permission });
}
