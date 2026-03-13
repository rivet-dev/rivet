import type { ActionContext } from "@/actor/contexts";
import { ACTOR_CONTEXT_INTERNAL_SYMBOL } from "@/actor/contexts/base/actor";
import type { DatabaseProvider } from "@/actor/database";
import { actor } from "@/actor/mod";
import type { RawAccess } from "@/db/config";
import type {
	SandboxAgent,
	Session,
	SessionPermissionRequest,
} from "sandbox-agent";
import { SqliteSessionPersistDriver } from "./persist";
import {
	type SandboxActorActions,
	type SandboxActorConfig,
	type SandboxActorCreateProviderContext,
	type SandboxActorPreventSleepOptions,
	type SandboxActorProvider,
	type SandboxActorRuntime,
	type SandboxActorState,
	type SandboxSessionEvent,
	SANDBOX_AGENT_ACTION_METHODS,
} from "./types";

type SandboxDatabase = DatabaseProvider<RawAccess>;

type SandboxActionContext<TConnParams, TInput> = ActionContext<
	SandboxActorState,
	TConnParams,
	undefined,
	SandboxActorRuntime,
	TInput,
	SandboxDatabase
>;

type SandboxActionDefinitions<TConnParams, TInput> = {
	[K in keyof SandboxActorActions]: (
		c: SandboxActionContext<TConnParams, TInput>,
		...args: Parameters<SandboxActorActions[K]>
	) => ReturnType<SandboxActorActions[K]>;
};

type ResolvedPreventSleepOptions = {
	warningAfterMs: number;
	staleAfterMs: number;
};

const DEFAULT_PREVENT_SLEEP_WARNING_AFTER_MS = 30_000;
const DEFAULT_PREVENT_SLEEP_STALE_AFTER_MS = 5 * 60_000;

function hasStaticProvider<TConnParams, TInput>(
	config: SandboxActorConfig<TConnParams, TInput>,
): config is SandboxActorConfig<TConnParams, TInput> & {
	provider: SandboxActorProvider;
} {
	return config.provider !== undefined;
}

function isSession(value: unknown): value is Session {
	return (
		typeof value === "object" &&
		value !== null &&
		"id" in value &&
		typeof (value as { id?: unknown }).id === "string"
	);
}

function isSessionArray(value: unknown): value is Session[] {
	return Array.isArray(value) && value.every(isSession);
}

function getPreventSleepOptions<TConnParams, TInput>(
	config: SandboxActorConfig<TConnParams, TInput>,
): ResolvedPreventSleepOptions | null {
	const raw = config.preventSleepWhileTurnsActive;
	if (!raw) {
		return null;
	}

	const options =
		raw === true
			? ({} satisfies SandboxActorPreventSleepOptions)
			: raw;
	const staleAfterMs = Math.max(
		1,
		Math.floor(
			options.staleAfterMs ?? DEFAULT_PREVENT_SLEEP_STALE_AFTER_MS,
		),
	);
	const warningAfterMs = Math.min(
		staleAfterMs,
		Math.max(
			0,
			Math.floor(
				options.warningAfterMs ??
					DEFAULT_PREVENT_SLEEP_WARNING_AFTER_MS,
			),
		),
	);

	return {
		warningAfterMs,
		staleAfterMs,
	};
}

function ensureTurnTrackingState(state: SandboxActorState): void {
	if (!Array.isArray(state.activeSessionIds)) {
		state.activeSessionIds = [];
	}
	if (!state.activePromptRequestIdsBySessionId) {
		state.activePromptRequestIdsBySessionId = {};
	}
	if (!state.activeSessionLastEventAtById) {
		state.activeSessionLastEventAtById = {};
	}
}

function removeString(list: string[], value: string): void {
	const index = list.indexOf(value);
	if (index >= 0) {
		list.splice(index, 1);
	}
}

function removeSessionId(
	state: SandboxActorState,
	sessionId: string,
): void {
	removeString(state.sessionIds, sessionId);
}

function addSessionId(state: SandboxActorState, sessionId: string): void {
	if (!state.sessionIds.includes(sessionId)) {
		state.sessionIds.push(sessionId);
	}
}

function clearSessionTimers(
	runtime: SandboxActorRuntime,
	sessionId: string,
): void {
	const warningTimeout = runtime.warningTimeoutBySessionId.get(sessionId);
	if (warningTimeout) {
		clearTimeout(warningTimeout);
		runtime.warningTimeoutBySessionId.delete(sessionId);
	}

	const staleTimeout = runtime.staleTimeoutBySessionId.get(sessionId);
	if (staleTimeout) {
		clearTimeout(staleTimeout);
		runtime.staleTimeoutBySessionId.delete(sessionId);
	}
}

function clearAllSessionTimers(runtime: SandboxActorRuntime): void {
	for (const sessionId of runtime.warningTimeoutBySessionId.keys()) {
		clearSessionTimers(runtime, sessionId);
	}
	for (const sessionId of runtime.staleTimeoutBySessionId.keys()) {
		clearSessionTimers(runtime, sessionId);
	}
}

function hasActiveSessions(state: SandboxActorState): boolean {
	ensureTurnTrackingState(state);
	return state.activeSessionIds.length > 0;
}

function syncPreventSleep<TConnParams, TInput>(
	c: SandboxActionContext<TConnParams, TInput>,
	options: ResolvedPreventSleepOptions | null,
): void {
	if (!options) {
		return;
	}

	c.setPreventSleep(c.vars.activeHookCount > 0 || hasActiveSessions(c.state));
}

function getPromptRequestIds(
	state: SandboxActorState,
	sessionId: string,
): string[] {
	ensureTurnTrackingState(state);
	return state.activePromptRequestIdsBySessionId[sessionId] ?? [];
}

function setPromptRequestIds(
	state: SandboxActorState,
	sessionId: string,
	requestIds: string[],
): void {
	ensureTurnTrackingState(state);
	if (requestIds.length === 0) {
		delete state.activePromptRequestIdsBySessionId[sessionId];
		return;
	}

	state.activePromptRequestIdsBySessionId[sessionId] = requestIds;
}

function markSessionActive<TConnParams, TInput>(
	c: SandboxActionContext<TConnParams, TInput>,
	options: ResolvedPreventSleepOptions | null,
	sessionId: string,
	requestId?: string,
): void {
	if (!options) {
		return;
	}

	ensureTurnTrackingState(c.state);

	if (!c.state.activeSessionIds.includes(sessionId)) {
		c.state.activeSessionIds.push(sessionId);
	}

	if (requestId) {
		const requestIds = getPromptRequestIds(c.state, sessionId);
		if (!requestIds.includes(requestId)) {
			requestIds.push(requestId);
			setPromptRequestIds(c.state, sessionId, requestIds);
		}
	}

	c.state.activeSessionLastEventAtById[sessionId] = Date.now();
	scheduleSessionTimers(c, options, sessionId);
	syncPreventSleep(c, options);
}

function touchSessionActive<TConnParams, TInput>(
	c: SandboxActionContext<TConnParams, TInput>,
	options: ResolvedPreventSleepOptions | null,
	sessionId: string,
): void {
	if (!options || !hasActiveSessions(c.state)) {
		return;
	}
	if (!c.state.activeSessionIds.includes(sessionId)) {
		return;
	}

	c.state.activeSessionLastEventAtById[sessionId] = Date.now();
	scheduleSessionTimers(c, options, sessionId);
}

function clearSessionActive<TConnParams, TInput>(
	c: SandboxActionContext<TConnParams, TInput>,
	options: ResolvedPreventSleepOptions | null,
	sessionId: string,
	requestId?: string,
): void {
	if (!options) {
		return;
	}

	ensureTurnTrackingState(c.state);

	if (requestId) {
		const remaining = getPromptRequestIds(c.state, sessionId).filter(
			(activeRequestId) => activeRequestId !== requestId,
		);
		setPromptRequestIds(c.state, sessionId, remaining);
		if (remaining.length > 0) {
			touchSessionActive(c, options, sessionId);
			syncPreventSleep(c, options);
			return;
		}
	}

	removeString(c.state.activeSessionIds, sessionId);
	delete c.state.activePromptRequestIdsBySessionId[sessionId];
	delete c.state.activeSessionLastEventAtById[sessionId];
	clearSessionTimers(c.vars, sessionId);
	syncPreventSleep(c, options);
}

function clearAllActiveSessions<TConnParams, TInput>(
	c: SandboxActionContext<TConnParams, TInput>,
	options: ResolvedPreventSleepOptions | null,
): void {
	if (!options) {
		return;
	}

	ensureTurnTrackingState(c.state);
	c.state.activeSessionIds = [];
	c.state.activePromptRequestIdsBySessionId = {};
	c.state.activeSessionLastEventAtById = {};
	clearAllSessionTimers(c.vars);
	syncPreventSleep(c, options);
}

function scheduleSessionTimers<TConnParams, TInput>(
	c: SandboxActionContext<TConnParams, TInput>,
	options: ResolvedPreventSleepOptions,
	sessionId: string,
): void {
	clearSessionTimers(c.vars, sessionId);

	const lastEventAt = c.state.activeSessionLastEventAtById[sessionId];
	if (!lastEventAt) {
		return;
	}

	// Sandbox session subscriptions are outbound SDK streams, not actor
	// connections. We keep the actor awake ourselves while a session appears to
	// be in the middle of a turn, then warn and eventually clear the state if the
	// stream goes quiet and we never observe a terminal response.
	const warningDelay = Math.max(
		0,
		options.warningAfterMs - (Date.now() - lastEventAt),
	);
	c.vars.warningTimeoutBySessionId.set(
		sessionId,
		setTimeout(() => {
			if (!c.state.activeSessionIds.includes(sessionId)) {
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
			if (!c.state.activeSessionIds.includes(sessionId)) {
				return;
			}

			c.log.warn({
				msg: "sandbox actor cleared stale active turn state after inactivity timeout",
				sessionId,
				idleMs: Date.now() - lastEventAt,
			});
			clearSessionActive(c, options, sessionId);
		}, staleDelay),
	);
}

function payloadMethod(payload: unknown): string | null {
	if (
		typeof payload !== "object" ||
		payload === null ||
		!("method" in payload)
	) {
		return null;
	}

	return typeof (payload as { method?: unknown }).method === "string"
		? (payload as { method: string }).method
		: null;
}

function payloadId(payload: unknown): string | null {
	if (
		typeof payload !== "object" ||
		payload === null ||
		!("id" in payload)
	) {
		return null;
	}

	const id = (payload as { id?: unknown }).id;
	if (typeof id === "string" || typeof id === "number") {
		return String(id);
	}
	return null;
}

function trackSessionTurnFromEvent<TConnParams, TInput>(
	c: SandboxActionContext<TConnParams, TInput>,
	options: ResolvedPreventSleepOptions | null,
	sessionId: string,
	event: SandboxSessionEvent,
): void {
	if (!options) {
		return;
	}

	const method = payloadMethod(event.payload);
	const id = payloadId(event.payload);

	if (event.sender === "client" && method === "session/prompt") {
		// The sandbox-agent stream gives us the raw JSON-RPC envelopes for the
		// session. We treat an observed prompt request as the start of an active
		// turn and keep the actor awake until we see the matching response or the
		// stale timeout clears the session.
		markSessionActive(
			c,
			options,
			sessionId,
			id ?? `session-prompt:${event.id}`,
		);
		return;
	}

	if (!c.state.activeSessionIds.includes(sessionId)) {
		return;
	}

	if (event.sender === "agent" && id) {
		const requestIds = getPromptRequestIds(c.state, sessionId);
		if (requestIds.length === 0 || requestIds.includes(id)) {
			clearSessionActive(c, options, sessionId, id);
			return;
		}
	}

	touchSessionActive(c, options, sessionId);
}

function runHook<TConnParams, TInput>(
	c: SandboxActionContext<TConnParams, TInput>,
	options: ResolvedPreventSleepOptions | null,
	sessionId: string,
	name: "onSessionEvent" | "onPermissionRequest",
	callback: () => void | Promise<void>,
): void {
	if (options) {
		c.vars.activeHookCount++;
		syncPreventSleep(c, options);
	}

	const promise = Promise.resolve(callback())
		.catch((error) => {
			c.log.error({
				msg: `sandbox actor ${name} hook failed`,
				sessionId,
				error,
			});
		})
		.finally(() => {
			if (!options) {
				return;
			}

			c.vars.activeHookCount--;
			if (c.vars.activeHookCount < 0) {
				c.vars.activeHookCount = 0;
				c.log.warn({
					msg: "sandbox actor active hook count went below 0",
				});
			}
			syncPreventSleep(c, options);
		});

	c.waitUntil(promise);
}

function getActorInput<TInput>(
	c: SandboxActionContext<unknown, TInput>,
): TInput {
	const actor = c[
		ACTOR_CONTEXT_INTERNAL_SYMBOL
	] as SandboxActionContext<unknown, TInput>[typeof ACTOR_CONTEXT_INTERNAL_SYMBOL] & {
		stateManager: {
			persist: {
				input: TInput;
			};
		};
	};

	return actor.stateManager.persist.input;
}

async function teardownAgentRuntime(
	runtime: SandboxActorRuntime,
): Promise<void> {
	for (const subscription of runtime.unsubscribeBySessionId.values()) {
		subscription.event?.();
		subscription.permission?.();
	}
	runtime.unsubscribeBySessionId.clear();
	clearAllSessionTimers(runtime);
	runtime.activeHookCount = 0;

	if (runtime.agent) {
		try {
			await runtime.agent.dispose();
		} finally {
			runtime.agent = null;
		}
	}

	runtime.provider = null;
}

async function createAgent(
	provider: SandboxActorProvider,
	sandboxId: string,
	dbClient: RawAccess,
	persistRawEvents: boolean,
): Promise<SandboxAgent> {
	return await provider.connectAgent(sandboxId, {
		persist: new SqliteSessionPersistDriver(dbClient, persistRawEvents),
		waitForHealth: true,
	});
}

async function resolveProvider<TConnParams, TInput>(
	c: SandboxActionContext<TConnParams, TInput>,
	config: SandboxActorConfig<TConnParams, TInput>,
): Promise<SandboxActorProvider> {
	if (c.vars.provider) {
		return c.vars.provider;
	}

	const provider =
		hasStaticProvider(config)
			? config.provider
			: await config.createProvider(
					c as unknown as SandboxActorCreateProviderContext<TInput>,
					getActorInput(c),
				);

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

async function ensureAgent<TConnParams, TInput>(
	c: SandboxActionContext<TConnParams, TInput>,
	config: SandboxActorConfig<TConnParams, TInput>,
	persistRawEvents: boolean,
): Promise<SandboxAgent> {
	if (c.vars.agent) {
		return c.vars.agent;
	}

	const provider = await resolveProvider(c, config);

	if (!c.state.sandboxId) {
		c.state.sandboxId = await provider.create({
			actorId: c.actorId,
			actorKey: [...c.key],
		});
	}

	c.vars.agent = await createAgent(
		provider,
		c.state.sandboxId,
		c.db,
		persistRawEvents,
	);

	for (const sessionId of c.state.sessionIds) {
		subscribeToSession(c, config, sessionId);
	}

	return c.vars.agent;
}

function subscribeToSession<TConnParams, TInput>(
	c: SandboxActionContext<TConnParams, TInput>,
	config: SandboxActorConfig<TConnParams, TInput>,
	sessionId: string,
): void {
	if (c.vars.unsubscribeBySessionId.has(sessionId)) {
		return;
	}

	const agent = c.vars.agent;
	if (!agent) {
		return;
	}

	const preventSleepOptions = getPreventSleepOptions(config);

	const event = agent.onSessionEvent(sessionId, (sessionEvent) => {
		trackSessionTurnFromEvent(
			c,
			preventSleepOptions,
			sessionId,
			sessionEvent,
		);

		if (!config.onSessionEvent) {
			return;
		}

		runHook(c, preventSleepOptions, sessionId, "onSessionEvent", () =>
			config.onSessionEvent!(c, sessionId, sessionEvent),
		);
	});

	const permission = agent.onPermissionRequest(
		sessionId,
		(request: SessionPermissionRequest) => {
			markSessionActive(c, preventSleepOptions, sessionId);

			if (!config.onPermissionRequest) {
				return;
			}

			runHook(
				c,
				preventSleepOptions,
				sessionId,
				"onPermissionRequest",
				() => config.onPermissionRequest!(c, sessionId, request),
			);
		},
	);

	c.vars.unsubscribeBySessionId.set(sessionId, { event, permission });
	addSessionId(c.state, sessionId);
}

async function afterAction<TConnParams, TInput>(
	c: SandboxActionContext<TConnParams, TInput>,
	config: SandboxActorConfig<TConnParams, TInput>,
	actionName: keyof SandboxActorActions,
	result: unknown,
): Promise<void> {
	const preventSleepOptions = getPreventSleepOptions(config);

	if (actionName === "dispose") {
		await teardownAgentRuntime(c.vars);
		clearAllActiveSessions(c, preventSleepOptions);
		c.setPreventSleep(false);
		return;
	}

	if (actionName === "destroySession" && isSession(result)) {
		c.vars.unsubscribeBySessionId.get(result.id)?.event?.();
		c.vars.unsubscribeBySessionId.get(result.id)?.permission?.();
		c.vars.unsubscribeBySessionId.delete(result.id);
		removeSessionId(c.state, result.id);
		clearSessionActive(c, preventSleepOptions, result.id);
		return;
	}

	if (
		(actionName === "createSession" ||
			actionName === "resumeSession" ||
			actionName === "resumeOrCreateSession" ||
			actionName === "getSession") &&
		isSession(result)
	) {
		subscribeToSession(c, config, result.id);
		return;
	}

	if (actionName === "listSessions" && result && typeof result === "object") {
		const items = (result as { items?: unknown }).items;
		if (isSessionArray(items)) {
			for (const session of items) {
				subscribeToSession(c, config, session.id);
			}
		}
	}
}

async function invokeAction<TConnParams, TInput>(
	c: SandboxActionContext<TConnParams, TInput>,
	config: SandboxActorConfig<TConnParams, TInput>,
	actionName: keyof SandboxActorActions,
	args: unknown[],
): Promise<unknown> {
	const agent = await ensureAgent(
		c,
		config,
		config.persistRawEvents ?? false,
	);
	const method = agent[actionName] as (...innerArgs: unknown[]) => unknown;
	const result = await method.apply(agent, args);
	await afterAction(c, config, actionName, result);
	return result;
}

function createAction<
	TConnParams,
	TInput,
	K extends keyof SandboxActorActions,
>(
	config: SandboxActorConfig<TConnParams, TInput>,
	actionName: K,
): SandboxActionDefinitions<TConnParams, TInput>[K] {
	return (async (
		c: SandboxActionContext<TConnParams, TInput>,
		...args: Parameters<SandboxActorActions[K]>
	) => {
		return await invokeAction(c, config, actionName, args);
	}) as SandboxActionDefinitions<TConnParams, TInput>[K];
}

function buildActions<TConnParams, TInput>(
	config: SandboxActorConfig<TConnParams, TInput>,
): SandboxActionDefinitions<TConnParams, TInput> {
	return Object.fromEntries(
		SANDBOX_AGENT_ACTION_METHODS.map((actionName) => [
			actionName,
			createAction(config, actionName),
		]),
	) as SandboxActionDefinitions<TConnParams, TInput>;
}

export function sandboxActor<TConnParams = undefined, TInput = undefined>(
	config: SandboxActorConfig<TConnParams, TInput>,
) {
	return actor<
		SandboxActorState,
		TConnParams,
		undefined,
		SandboxActorRuntime,
		TInput,
		SandboxDatabase,
		Record<never, never>,
		Record<never, never>,
		SandboxActionDefinitions<TConnParams, TInput>
	>({
		createState: async (c, input) => ({
			sandboxId: null,
			sessionIds: [],
			providerName:
				hasStaticProvider(config)
					? config.provider.name
					: (await config.createProvider(
							c as SandboxActorCreateProviderContext<TInput>,
							input,
						)).name,
			activeSessionIds: [],
			activePromptRequestIdsBySessionId: {},
			activeSessionLastEventAtById: {},
		}),
		createVars: () => ({
			agent: null,
			provider: null,
			unsubscribeBySessionId: new Map(),
			activeHookCount: 0,
			warningTimeoutBySessionId: new Map(),
			staleTimeoutBySessionId: new Map(),
		}),
		db: config.database,
		onWake: async (c) => {
			const preventSleepOptions = getPreventSleepOptions(config);
			ensureTurnTrackingState(c.state);
			if (preventSleepOptions) {
				for (const sessionId of c.state.activeSessionIds) {
					scheduleSessionTimers(
						c as SandboxActionContext<TConnParams, TInput>,
						preventSleepOptions,
						sessionId,
					);
				}
			}
			syncPreventSleep(
				c as SandboxActionContext<TConnParams, TInput>,
				preventSleepOptions,
			);

			if (!c.state.sandboxId) {
				return;
			}

			const provider = await resolveProvider(
				c as SandboxActionContext<TConnParams, TInput>,
				config,
			);
			c.vars.agent = await createAgent(
				provider,
				c.state.sandboxId,
				c.db,
				config.persistRawEvents ?? false,
			);

			for (const sessionId of c.state.sessionIds) {
				subscribeToSession(
					c as SandboxActionContext<TConnParams, TInput>,
					config,
					sessionId,
				);
			}
		},
		onSleep: async (c) => {
			await teardownAgentRuntime(c.vars);
			c.setPreventSleep(false);
		},
		onDestroy: async (c) => {
			const preventSleepOptions = getPreventSleepOptions(config);
			clearAllActiveSessions(
				c as SandboxActionContext<TConnParams, TInput>,
				preventSleepOptions,
			);
			await teardownAgentRuntime(c.vars);
			c.setPreventSleep(false);
			if (!c.state.sandboxId) {
				return;
			}

			try {
				const provider = await resolveProvider(
					c as SandboxActionContext<TConnParams, TInput>,
					config,
				);
				await provider.destroy(c.state.sandboxId);
			} finally {
				c.state.sandboxId = null;
				c.state.sessionIds = [];
				c.state.providerName = null;
				c.state.activeSessionIds = [];
				c.state.activePromptRequestIdsBySessionId = {};
				c.state.activeSessionLastEventAtById = {};
			}
		},
		onBeforeConnect: config.onBeforeConnect,
		actions: buildActions(config),
	});
}
