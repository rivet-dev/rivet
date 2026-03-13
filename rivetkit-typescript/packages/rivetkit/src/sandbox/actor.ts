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
	type SandboxActorProvider,
	type SandboxActorRuntime,
	type SandboxActorState,
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

function removeSessionId(
	state: SandboxActorState,
	sessionId: string,
): void {
	const index = state.sessionIds.indexOf(sessionId);
	if (index >= 0) {
		state.sessionIds.splice(index, 1);
	}
}

function addSessionId(state: SandboxActorState, sessionId: string): void {
	if (!state.sessionIds.includes(sessionId)) {
		state.sessionIds.push(sessionId);
	}
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

	const event = agent.onSessionEvent(sessionId, (sessionEvent) => {
		if (!config.onSessionEvent) {
			return;
		}

		c.waitUntil(
			Promise.resolve(config.onSessionEvent(c, sessionId, sessionEvent)).catch(
				(error) => {
					c.log.error({
						msg: "sandbox actor onSessionEvent hook failed",
						sessionId,
						error,
					});
				},
			),
		);
	});

	const permission = agent.onPermissionRequest(
		sessionId,
		(request: SessionPermissionRequest) => {
			if (!config.onPermissionRequest) {
				return;
			}

			c.waitUntil(
				Promise.resolve(
					config.onPermissionRequest(c, sessionId, request),
				).catch((error) => {
					c.log.error({
						msg: "sandbox actor onPermissionRequest hook failed",
						sessionId,
						error,
					});
				}),
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
	if (actionName === "dispose") {
		await teardownAgentRuntime(c.vars);
		return;
	}

	if (actionName === "destroySession" && isSession(result)) {
		c.vars.unsubscribeBySessionId.get(result.id)?.event?.();
		c.vars.unsubscribeBySessionId.get(result.id)?.permission?.();
		c.vars.unsubscribeBySessionId.delete(result.id);
		removeSessionId(c.state, result.id);
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
		}),
		createVars: () => ({
			agent: null,
			provider: null,
			unsubscribeBySessionId: new Map(),
		}),
		db: config.database,
		onWake: async (c) => {
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
		},
		onDestroy: async (c) => {
			await teardownAgentRuntime(c.vars);
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
			}
		},
		onBeforeConnect: config.onBeforeConnect,
		actions: buildActions(config),
	});
}
