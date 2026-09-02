import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
	type AgentSession,
	type AgentSessionEvent,
	createExtensionRuntime,
	createAgentSession,
	type CreateAgentSessionOptions,
	type PromptOptions,
	type ResourceLoader,
	SessionManager,
	SettingsManager,
} from "@earendil-works/pi-coding-agent";
import type {
	Sandbox,
	SandboxActorContext,
	SandboxAdapter,
	SandboxBinding,
} from "@rivet-dev/sandbox-adapter";
import {
	type Actions,
	type ActorConfigInput,
	type ActorContext,
	type ActorDefinition,
	actor,
	event,
	type Type,
} from "rivetkit";
import { type DatabaseProvider, db, type RawAccess } from "rivetkit/db";
import {
	createSandboxBashOperations,
	createSandboxTools,
} from "./sandbox-tools.js";
import {
	loadPiSession,
	migratePiActorTables,
	savePiSession,
	serializeSession,
	type StoredPiSession,
} from "./storage.js";

const DEFAULT_ACTION_TIMEOUT_MS = 2_147_483_647;
const DEFAULT_SLEEP_GRACE_PERIOD_MS = 15 * 60_000;

type ActorDb = DatabaseProvider<RawAccess>;
type EventSchemaConfig = Record<string, any>;
type QueueSchemaConfig = Record<string, any>;
type AnyContext = ActorContext<any, any, any, any, any, ActorDb, any, any> &
	SandboxActorContext;
type BuiltInEvents = { event: Type<AgentSessionEvent> };

const builtInEvents: BuiltInEvents = {
	event: event<AgentSessionEvent>(),
};

interface PiRuntime {
	ready: Promise<PiRuntime>;
	session?: AgentSession;
	manager?: SessionManager;
	settingsManager?: SettingsManager;
	ownsSettingsManager: boolean;
	sandbox?: Sandbox;
	binding?: SandboxBinding;
	cwd?: string;
	tempDir?: string;
	unsubscribe?: () => void;
	persistTail: Promise<void>;
	checkpointScheduled: boolean;
	shutdownEmitted: boolean;
}

const runtimes = new Map<string, PiRuntime>();
const disposingActors = new Set<string>();

export interface PiActorExtras
	extends Omit<CreateAgentSessionOptions, "sessionManager" | "cwd"> {
	/** Working directory on the host, or requested directory in the sandbox. */
	cwd?: string;
	/** Optional provider that mounts an isolated filesystem and command runner. */
	sandbox?: SandboxAdapter<AnyContext>;
}

export interface PiActorEventHooks<TContext = AnyContext> {
	/** Receives the same events broadcast through the actor's `event` event. */
	onSessionEvent?: (
		context: TContext,
		event: AgentSessionEvent,
	) => void | Promise<void>;
}

type DistributiveOmit<T, K extends PropertyKey> = T extends unknown
	? Omit<T, K>
	: never;

export type PiActorConfigInput<
	TState = undefined,
	TConnParams = undefined,
	TConnState = undefined,
	TVars = undefined,
	TInput = undefined,
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
	TUserActions extends Actions<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		ActorDb,
		TEvents,
		TQueues
	> = Record<never, never>,
> = DistributiveOmit<
	ActorConfigInput<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		ActorDb,
		TEvents,
		TQueues,
		TUserActions
	>,
	"db"
> &
	PiActorExtras &
	PiActorEventHooks<
		ActorContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			ActorDb,
			TEvents,
			TQueues
		>
	>;

const piOptionKeys = [
	"cwd",
	"agentDir",
	"modelRuntime",
	"model",
	"thinkingLevel",
	"scopedModels",
	"noTools",
	"tools",
	"excludeTools",
	"customTools",
	"resourceLoader",
	"settingsManager",
	"sessionStartEvent",
] as const satisfies readonly (keyof CreateAgentSessionOptions)[];

function splitConfig(
	config: PiActorConfigInput<any, any, any, any, any, any, any, any>,
): {
	actorConfig: Record<string, unknown>;
	sessionOptions: CreateAgentSessionOptions;
	sandbox?: SandboxAdapter<AnyContext>;
	onSessionEvent?: PiActorEventHooks["onSessionEvent"];
} {
	const actorConfig = { ...config } as Record<string, unknown>;
	const sessionOptions: CreateAgentSessionOptions = {};
	for (const key of piOptionKeys) {
		if (key in actorConfig) {
			(sessionOptions as Record<string, unknown>)[key] = actorConfig[key];
			delete actorConfig[key];
		}
	}
	const sandbox = actorConfig.sandbox as SandboxAdapter<AnyContext> | undefined;
	const onSessionEvent = actorConfig.onSessionEvent as
		| PiActorEventHooks["onSessionEvent"]
		| undefined;
	delete actorConfig.sandbox;
	delete actorConfig.onSessionEvent;
	return { actorConfig, sessionOptions, sandbox, onSessionEvent };
}

type SessionMethodAction<K extends keyof AgentSession> = AgentSession[K] extends (
	...args: infer TArgs
) => infer TResult
	? (context: AnyContext, ...args: TArgs) => Promise<Awaited<TResult>>
	: never;

export interface PiBashResult {
	output: string;
	exitCode: number | undefined;
	cancelled: boolean;
	truncated: boolean;
	fullOutputPath?: string;
}

export interface PiSessionInfo {
	sessionId: string;
	sessionName?: string;
	cwd?: string;
	model: AgentSession["model"];
	thinkingLevel: AgentSession["thinkingLevel"];
	isStreaming: boolean;
	isIdle: boolean;
	isCompacting: boolean;
	isRetrying: boolean;
	isBashRunning: boolean;
	pendingMessageCount: number;
	activeTools: string[];
	steeringMode: AgentSession["steeringMode"];
	followUpMode: AgentSession["followUpMode"];
	autoCompactionEnabled: boolean;
	autoRetryEnabled: boolean;
	retryAttempt: number;
}

export type PiPromptOptions = Omit<PromptOptions, "preflightResult">;

export type PiActions = {
	prompt: (
		context: AnyContext,
		text: string,
		options?: PiPromptOptions,
	) => Promise<void>;
	steer: SessionMethodAction<"steer">;
	followUp: SessionMethodAction<"followUp">;
	sendCustomMessage: SessionMethodAction<"sendCustomMessage">;
	sendUserMessage: SessionMethodAction<"sendUserMessage">;
	clearQueue: SessionMethodAction<"clearQueue">;
	abort: SessionMethodAction<"abort">;
	waitForIdle: SessionMethodAction<"waitForIdle">;
	getActiveToolNames: SessionMethodAction<"getActiveToolNames">;
	getAllTools: (
		context: AnyContext,
	) => Promise<ReturnType<AgentSession["getAllTools"]>>;
	setActiveToolsByName: SessionMethodAction<"setActiveToolsByName">;
	setScopedModels: SessionMethodAction<"setScopedModels">;
	setModel: SessionMethodAction<"setModel">;
	cycleModel: SessionMethodAction<"cycleModel">;
	setThinkingLevel: SessionMethodAction<"setThinkingLevel">;
	cycleThinkingLevel: SessionMethodAction<"cycleThinkingLevel">;
	getAvailableThinkingLevels: SessionMethodAction<"getAvailableThinkingLevels">;
	supportsThinking: SessionMethodAction<"supportsThinking">;
	setSteeringMode: SessionMethodAction<"setSteeringMode">;
	setFollowUpMode: SessionMethodAction<"setFollowUpMode">;
	compact: SessionMethodAction<"compact">;
	abortCompaction: SessionMethodAction<"abortCompaction">;
	abortBranchSummary: SessionMethodAction<"abortBranchSummary">;
	setAutoCompactionEnabled: SessionMethodAction<"setAutoCompactionEnabled">;
	reload: (context: AnyContext) => Promise<void>;
	abortRetry: SessionMethodAction<"abortRetry">;
	setAutoRetryEnabled: SessionMethodAction<"setAutoRetryEnabled">;
	executeBash: (
		context: AnyContext,
		command: string,
		options?: { excludeFromContext?: boolean; id?: string },
	) => Promise<PiBashResult>;
	abortBash: SessionMethodAction<"abortBash">;
	recordBashResult: SessionMethodAction<"recordBashResult">;
	setSessionName: SessionMethodAction<"setSessionName">;
	navigateTree: SessionMethodAction<"navigateTree">;
	getUserMessagesForForking: SessionMethodAction<"getUserMessagesForForking">;
	getSessionStats: SessionMethodAction<"getSessionStats">;
	getContextUsage: SessionMethodAction<"getContextUsage">;
	getLastAssistantText: SessionMethodAction<"getLastAssistantText">;
	getSteeringMessages: SessionMethodAction<"getSteeringMessages">;
	getFollowUpMessages: SessionMethodAction<"getFollowUpMessages">;
	getMessages: (context: AnyContext) => Promise<AgentSession["messages"]>;
	getSessionTree: (
		context: AnyContext,
	) => Promise<ReturnType<SessionManager["getTree"]>>;
	getSession: (context: AnyContext) => Promise<PiSessionInfo>;
};

function createPiActions(
	sessionOptions: CreateAgentSessionOptions,
	sandboxAdapter: SandboxAdapter<AnyContext> | undefined,
	onSessionEvent: PiActorEventHooks["onSessionEvent"],
): PiActions {
	const read = <T>(
		context: AnyContext,
		callback: (session: AgentSession, runtime: PiRuntime) => T | Promise<T>,
	): Promise<T> =>
		context.keepAwake(
			ensureRuntime(
				context,
				sessionOptions,
				sandboxAdapter,
				onSessionEvent,
			).then((runtime) => callback(runtime.session!, runtime)),
		);
	const mutate = <T>(
		context: AnyContext,
		callback: (session: AgentSession, runtime: PiRuntime) => T | Promise<T>,
	): Promise<T> =>
		context.keepAwake(
			(async () => {
				const runtime = await ensureRuntime(
					context,
					sessionOptions,
					sandboxAdapter,
					onSessionEvent,
				);
				try {
					return await callback(runtime.session!, runtime);
				} finally {
					await persistRuntime(context, runtime);
				}
			})(),
		);

	return {
		prompt: (c: AnyContext, text: string, options?: PiPromptOptions) =>
			mutate(c, (session) => session.prompt(text, options)),
		steer: (c: AnyContext, ...args: Parameters<AgentSession["steer"]>) =>
			mutate(c, (session) => session.steer(...args)),
		followUp: (
			c: AnyContext,
			...args: Parameters<AgentSession["followUp"]>
		) => mutate(c, (session) => session.followUp(...args)),
		sendCustomMessage: (
			c: AnyContext,
			...args: Parameters<AgentSession["sendCustomMessage"]>
		) => mutate(c, (session) => session.sendCustomMessage(...args)),
		sendUserMessage: (
			c: AnyContext,
			...args: Parameters<AgentSession["sendUserMessage"]>
		) => mutate(c, (session) => session.sendUserMessage(...args)),
		clearQueue: (c: AnyContext) => mutate(c, (session) => session.clearQueue()),
		abort: (c: AnyContext) => mutate(c, (session) => session.abort()),
		waitForIdle: (c: AnyContext) =>
			mutate(c, (session) => session.waitForIdle()),
		getActiveToolNames: (c: AnyContext) =>
			read(c, (session) => session.getActiveToolNames()),
		getAllTools: (c: AnyContext) =>
			read(c, (session) => serializable(session.getAllTools())),
		setActiveToolsByName: (
			c: AnyContext,
			...args: Parameters<AgentSession["setActiveToolsByName"]>
		) => mutate(c, (session) => session.setActiveToolsByName(...args)),
		setScopedModels: (
			c: AnyContext,
			...args: Parameters<AgentSession["setScopedModels"]>
		) => mutate(c, (session) => session.setScopedModels(...args)),
		setModel: (c: AnyContext, ...args: Parameters<AgentSession["setModel"]>) =>
			mutate(c, (session) => session.setModel(...args)),
		cycleModel: (
			c: AnyContext,
			...args: Parameters<AgentSession["cycleModel"]>
		) => mutate(c, (session) => session.cycleModel(...args)),
		setThinkingLevel: (
			c: AnyContext,
			...args: Parameters<AgentSession["setThinkingLevel"]>
		) => mutate(c, (session) => session.setThinkingLevel(...args)),
		cycleThinkingLevel: (
			c: AnyContext,
			...args: Parameters<AgentSession["cycleThinkingLevel"]>
		) => mutate(c, (session) => session.cycleThinkingLevel(...args)),
		getAvailableThinkingLevels: (c: AnyContext) =>
			read(c, (session) => session.getAvailableThinkingLevels()),
		supportsThinking: (c: AnyContext) =>
			read(c, (session) => session.supportsThinking()),
		setSteeringMode: (
			c: AnyContext,
			...args: Parameters<AgentSession["setSteeringMode"]>
		) => mutate(c, (session) => session.setSteeringMode(...args)),
		setFollowUpMode: (
			c: AnyContext,
			...args: Parameters<AgentSession["setFollowUpMode"]>
		) => mutate(c, (session) => session.setFollowUpMode(...args)),
		compact: (c: AnyContext, ...args: Parameters<AgentSession["compact"]>) =>
			mutate(c, (session) => session.compact(...args)),
		abortCompaction: (c: AnyContext) =>
			mutate(c, (session) => session.abortCompaction()),
		abortBranchSummary: (c: AnyContext) =>
			mutate(c, (session) => session.abortBranchSummary()),
		setAutoCompactionEnabled: (
			c: AnyContext,
			...args: Parameters<AgentSession["setAutoCompactionEnabled"]>
		) => mutate(c, (session) => session.setAutoCompactionEnabled(...args)),
		reload: (c: AnyContext) => mutate(c, (session) => session.reload()),
		abortRetry: (c: AnyContext) =>
			mutate(c, (session) => session.abortRetry()),
		setAutoRetryEnabled: (
			c: AnyContext,
			...args: Parameters<AgentSession["setAutoRetryEnabled"]>
		) => mutate(c, (session) => session.setAutoRetryEnabled(...args)),
		executeBash: (
			c: AnyContext,
			command: string,
			options?: { excludeFromContext?: boolean; id?: string },
		) =>
			mutate(c, (session, runtime) =>
				session.executeBash(command, undefined, {
					...options,
					operations: runtime.sandbox
						? createSandboxBashOperations(runtime.sandbox)
						: undefined,
				}),
			),
		abortBash: (c: AnyContext) =>
			mutate(c, (session) => session.abortBash()),
		recordBashResult: (
			c: AnyContext,
			...args: Parameters<AgentSession["recordBashResult"]>
		) => mutate(c, (session) => session.recordBashResult(...args)),
		setSessionName: (
			c: AnyContext,
			...args: Parameters<AgentSession["setSessionName"]>
		) => mutate(c, (session) => session.setSessionName(...args)),
		navigateTree: (
			c: AnyContext,
			...args: Parameters<AgentSession["navigateTree"]>
		) => mutate(c, (session) => session.navigateTree(...args)),
		getUserMessagesForForking: (c: AnyContext) =>
			read(c, (session) => session.getUserMessagesForForking()),
		getSessionStats: (c: AnyContext) =>
			read(c, (session) => session.getSessionStats()),
		getContextUsage: (c: AnyContext) =>
			read(c, (session) => session.getContextUsage()),
		getLastAssistantText: (c: AnyContext) =>
			read(c, (session) => session.getLastAssistantText()),
		getSteeringMessages: (c: AnyContext) =>
			read(c, (session) => [...session.getSteeringMessages()]),
		getFollowUpMessages: (c: AnyContext) =>
			read(c, (session) => [...session.getFollowUpMessages()]),
		getMessages: (c: AnyContext) =>
			read(c, (session) => serializable(session.messages)),
		getSessionTree: (c: AnyContext) =>
			read(c, (session) => serializable(session.sessionManager.getTree())),
		getSession: (c: AnyContext) =>
			read(c, (session, runtime) => ({
				sessionId: session.sessionId,
				sessionName: session.sessionName,
				cwd: runtime.cwd,
				model: serializable(session.model),
				thinkingLevel: session.thinkingLevel,
				isStreaming: session.isStreaming,
				isIdle: session.isIdle,
				isCompacting: session.isCompacting,
				isRetrying: session.isRetrying,
				isBashRunning: session.isBashRunning,
				pendingMessageCount: session.pendingMessageCount,
				activeTools: session.getActiveToolNames(),
				steeringMode: session.steeringMode,
				followUpMode: session.followUpMode,
				autoCompactionEnabled: session.autoCompactionEnabled,
				autoRetryEnabled: session.autoRetryEnabled,
				retryAttempt: session.retryAttempt,
			})),
	};
}

export function pi<
	TState = undefined,
	TConnParams = undefined,
	TConnState = undefined,
	TVars = undefined,
	TInput = undefined,
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
	TUserActions extends Actions<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		ActorDb,
		TEvents,
		TQueues
	> = Record<never, never>,
>(
	config: PiActorConfigInput<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TEvents,
		TQueues,
		TUserActions
	> = {} as PiActorConfigInput<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TEvents,
		TQueues,
		TUserActions
	>,
): ActorDefinition<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	ActorDb,
	TEvents & BuiltInEvents,
	TQueues,
	TUserActions & PiActions
> {
	const split = splitConfig(config);
	const actorConfig = split.actorConfig as Omit<
		typeof config,
		keyof PiActorExtras | keyof PiActorEventHooks
	>;
	const actions = createPiActions(
		split.sessionOptions,
		split.sandbox,
		split.onSessionEvent,
	);
	assertNoReservedKeys("action", actorConfig.actions, actions);
	assertNoReservedKeys("event", actorConfig.events, builtInEvents);

	const userOnWake = actorConfig.onWake;
	const userOnSleep = actorConfig.onSleep;
	const userOnDestroy = actorConfig.onDestroy;

	return actor({
		...actorConfig,
		options: {
			actionTimeout: DEFAULT_ACTION_TIMEOUT_MS,
			sleepGracePeriod: DEFAULT_SLEEP_GRACE_PERIOD_MS,
			...actorConfig.options,
		},
		db: db({ onMigrate: migratePiActorTables }),
		events: { ...(actorConfig.events ?? {}), ...builtInEvents },
		actions: { ...(actorConfig.actions ?? {}), ...actions },
		onWake: async (context: AnyContext) => {
			disposingActors.delete(context.actorId);
			try {
				await userOnWake?.(context as never);
			} catch (error) {
				await disposeRuntime(context, split.sandbox, "error");
				throw error;
			}
		},
		onSleep: async (context: AnyContext) => {
			try {
				await userOnSleep?.(context as never);
			} finally {
				await disposeRuntime(context, split.sandbox, "sleep");
			}
		},
		onDestroy: async (context: AnyContext) => {
			try {
				await userOnDestroy?.(context as never);
			} finally {
				await disposeRuntime(context, split.sandbox, "destroy");
			}
		},
	} as any) as ActorDefinition<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		ActorDb,
		TEvents & BuiltInEvents,
		TQueues,
		TUserActions & PiActions
	>;
}

async function ensureRuntime(
	context: AnyContext,
	sessionOptions: CreateAgentSessionOptions,
	sandboxAdapter: SandboxAdapter<AnyContext> | undefined,
	onSessionEvent: PiActorEventHooks["onSessionEvent"],
): Promise<PiRuntime> {
	if (disposingActors.has(context.actorId)) {
		throw new Error("Pi actor is stopping");
	}
	const existing = runtimes.get(context.actorId);
	if (existing) return existing.ready;

	const runtime: PiRuntime = {
		ready: Promise.resolve(undefined as never),
		persistTail: Promise.resolve(),
		checkpointScheduled: false,
		ownsSettingsManager: false,
		shutdownEmitted: false,
	};
	runtimes.set(context.actorId, runtime);
	runtime.ready = initializeRuntime(
		context,
		runtime,
		sessionOptions,
		sandboxAdapter,
		onSessionEvent,
	).catch(async (error) => {
		if (runtimes.get(context.actorId) === runtime) {
			runtimes.delete(context.actorId);
		}
		const cleanupErrors: unknown[] = [];
		await captureError(cleanupErrors, () => shutdownRuntimeSession(runtime));
		await captureError(cleanupErrors, () => persistRuntime(context, runtime));
		if (sandboxAdapter && runtime.sandbox) {
			await captureError(cleanupErrors, () =>
				sandboxAdapter.suspend?.(context, {
					id: context.actorId,
					binding: runtime.binding,
					cwd: runtime.cwd,
					sandbox: runtime.sandbox,
				}),
			);
		}
		await captureError(cleanupErrors, () => cleanupRuntimeResources(runtime));
		if (cleanupErrors.length > 0) {
			context.log.error({
				msg: "failed to clean up an incomplete Pi session",
				error: cleanupErrors
					.map((cleanupError) =>
						cleanupError instanceof Error
							? cleanupError.message
							: String(cleanupError),
					)
					.join("; "),
			});
		}
		throw error;
	});
	return runtime.ready;
}

async function initializeRuntime(
	context: AnyContext,
	runtime: PiRuntime,
	sessionOptions: CreateAgentSessionOptions,
	sandboxAdapter: SandboxAdapter<AnyContext> | undefined,
	onSessionEvent: PiActorEventHooks["onSessionEvent"],
): Promise<PiRuntime> {
	const stored = await loadPiSession(context.db);
	let cwd = stored?.cwd ?? sessionOptions.cwd;
	if (sandboxAdapter) {
		runtime.sandbox = await sandboxAdapter.connect(context, {
			id: context.actorId,
			binding: stored?.sandboxBinding,
			cwd,
		});
		runtime.binding = runtime.sandbox.binding;
		cwd = runtime.sandbox.cwd;
	}
	cwd = cwd ?? process.cwd();
	if (!cwd.startsWith("/")) {
		throw new Error("Pi cwd must be an absolute path");
	}
	runtime.cwd = cwd;
	runtime.tempDir = await mkdtemp(join(tmpdir(), "rivet-pi-"));
	runtime.manager = await restoreSessionManager(stored, runtime.tempDir, cwd);
	runtime.ownsSettingsManager = sessionOptions.settingsManager === undefined;
	runtime.settingsManager =
		sessionOptions.settingsManager ?? SettingsManager.inMemory(stored?.settings);
	const resourceLoader =
		sessionOptions.resourceLoader ??
		(runtime.sandbox ? createEmptyResourceLoader() : undefined);

	const customTools = [
		...(sessionOptions.customTools ?? []),
		...(runtime.sandbox ? createSandboxTools(runtime.sandbox) : []),
	];
	const result = await createAgentSession({
		...sessionOptions,
		cwd,
		excludeTools: runtime.sandbox
			? [...new Set([...(sessionOptions.excludeTools ?? []), "powershell"])]
			: sessionOptions.excludeTools,
		customTools,
		sessionManager: runtime.manager,
		settingsManager: runtime.settingsManager,
		resourceLoader,
	});
	runtime.session = result.session;
	runtime.unsubscribe = result.session.subscribe((event) => {
		try {
			context.broadcast("event", serializable(event));
		} catch (error) {
			if (!isActorStoppingError(error)) {
				context.log.error({
					msg: "failed to broadcast Pi session event",
					error: error instanceof Error ? error.message : String(error),
				});
			}
		}
		if (onSessionEvent) {
			void context.keepAwake(
				Promise.resolve()
					.then(() => onSessionEvent(context, event))
					.catch((error) => {
						context.log.error({
							msg: "Pi session event hook failed",
							error: error instanceof Error ? error.message : String(error),
						});
					}),
			);
		}
		if (
			event.type === "turn_end" ||
			event.type === "agent_settled" ||
			event.type === "entry_appended"
		) {
			scheduleCheckpoint(context, runtime);
		}
	});
	await persistRuntime(context, runtime);
	return runtime;
}

async function restoreSessionManager(
	stored: StoredPiSession | undefined,
	tempDir: string,
	cwd: string,
): Promise<SessionManager> {
	if (!stored) return SessionManager.create(cwd, tempDir);
	const transcriptPath = join(tempDir, "session.jsonl");
	await writeFile(transcriptPath, stored.transcript, {
		encoding: "utf8",
		mode: 0o600,
	});
	const manager = SessionManager.open(transcriptPath, tempDir, cwd);
	if (manager.getSessionId() !== stored.sessionId) {
		throw new Error("Pi session ID does not match its persisted transcript");
	}
	return manager;
}

function scheduleCheckpoint(context: AnyContext, runtime: PiRuntime): void {
	if (runtime.checkpointScheduled) return;
	runtime.checkpointScheduled = true;
	queueMicrotask(() => {
		runtime.checkpointScheduled = false;
		context.waitUntil(
			persistRuntime(context, runtime).catch((error) => {
				context.log.error({
					msg: "failed to checkpoint Pi session",
					error: error instanceof Error ? error.message : String(error),
				});
			}),
		);
	});
}

function persistRuntime(context: AnyContext, runtime: PiRuntime): Promise<void> {
	const save = async () => {
		if (!runtime.manager || !runtime.cwd) return;
		if (runtime.ownsSettingsManager) {
			await runtime.settingsManager?.flush();
		}
		await savePiSession(context.db, {
			sessionId: runtime.manager.getSessionId(),
			cwd: runtime.cwd,
			transcript: serializeSession(runtime.manager),
			sandboxBinding: runtime.binding,
			settings: runtime.ownsSettingsManager
				? runtime.settingsManager?.getGlobalSettings()
				: undefined,
		});
	};
	runtime.persistTail = runtime.persistTail.then(save, save);
	return runtime.persistTail;
}

async function disposeRuntime(
	context: AnyContext,
	sandboxAdapter: SandboxAdapter<AnyContext> | undefined,
	reason: "sleep" | "destroy" | "error",
): Promise<void> {
	const runtime = runtimes.get(context.actorId);
	disposingActors.add(context.actorId);
	if (runtime) runtimes.delete(context.actorId);
	let readyRuntime: PiRuntime | undefined;
	const errors: unknown[] = [];
	if (runtime) {
		try {
			readyRuntime = await runtime.ready;
		} catch (error) {
			errors.push(error);
		}
	}
	if (readyRuntime?.session) {
		await captureError(errors, () => readyRuntime!.session!.abort());
		await captureError(errors, () => readyRuntime!.session!.waitForIdle());
		await captureError(errors, () => shutdownRuntimeSession(readyRuntime!));
	}
	if (readyRuntime) {
		await captureError(errors, () => persistRuntime(context, readyRuntime!));
	}

	let stored: StoredPiSession | undefined;
	if (!readyRuntime && sandboxAdapter) {
		await captureError(errors, async () => {
			stored = await loadPiSession(context.db);
		});
	}
	const binding = readyRuntime?.binding ?? stored?.sandboxBinding;
	const cwd = readyRuntime?.cwd ?? stored?.cwd;
	if (sandboxAdapter && (binding !== undefined || readyRuntime?.sandbox)) {
		const lifecycle = {
			id: context.actorId,
			binding,
			cwd,
			sandbox: readyRuntime?.sandbox,
		};
		await captureError(errors, () =>
			reason === "destroy"
				? (sandboxAdapter.destroy?.(context, lifecycle) ?? Promise.resolve())
				: (sandboxAdapter.suspend?.(context, lifecycle) ?? Promise.resolve()),
		);
	}
	if (readyRuntime) {
		await captureError(errors, () => cleanupRuntimeResources(readyRuntime!));
	}
	try {
		if (errors.length === 1) throw errors[0];
		if (errors.length > 1) {
			throw new AggregateError(errors, "Pi actor cleanup failed");
		}
	} finally {
		disposingActors.delete(context.actorId);
	}
}

async function cleanupRuntimeResources(runtime: PiRuntime): Promise<void> {
	runtime.unsubscribe?.();
	runtime.unsubscribe = undefined;
	runtime.session?.dispose();
	runtime.session = undefined;
	runtime.manager = undefined;
	runtime.settingsManager = undefined;
	if (runtime.tempDir) {
		const tempDir = runtime.tempDir;
		runtime.tempDir = undefined;
		await rm(tempDir, { recursive: true, force: true });
	}
}

async function shutdownRuntimeSession(runtime: PiRuntime): Promise<void> {
	if (!runtime.session || runtime.shutdownEmitted) return;
	runtime.shutdownEmitted = true;
	if (runtime.session.extensionRunner.hasHandlers("session_shutdown")) {
		await runtime.session.extensionRunner.emit({
			type: "session_shutdown",
			reason: "quit",
		});
	}
}

async function captureError(
	errors: unknown[],
	operation: () => void | Promise<void>,
): Promise<void> {
	try {
		await operation();
	} catch (error) {
		errors.push(error);
	}
}

function isActorStoppingError(error: unknown): boolean {
	if (!error || typeof error !== "object") return false;
	const candidate = error as { group?: unknown; code?: unknown };
	return candidate.group === "actor" && candidate.code === "stopping";
}

function createEmptyResourceLoader(): ResourceLoader {
	const extensions = {
		extensions: [],
		errors: [],
		runtime: createExtensionRuntime(),
	};
	return {
		getExtensions: () => extensions,
		getSkills: () => ({ skills: [], diagnostics: [] }),
		getPrompts: () => ({ prompts: [], diagnostics: [] }),
		getThemes: () => ({ themes: [], diagnostics: [] }),
		getAgentsFiles: () => ({ agentsFiles: [] }),
		getSystemPrompt: () => undefined,
		getSystemPromptSource: () => undefined,
		getAppendSystemPrompt: () => [],
		getAppendSystemPromptSources: () => [],
		extendResources: () => {},
		reload: async () => {},
	};
}

function assertNoReservedKeys(
	kind: string,
	custom: object | undefined,
	builtIns: object,
): void {
	for (const key of Object.keys(custom ?? {})) {
		if (key in builtIns) {
			throw new Error(`pi() ${kind} name is reserved: ${key}`);
		}
	}
}

function serializable(value: unknown): any {
	return sanitize(value, new WeakSet<object>());
}

function sanitize(value: unknown, seen: WeakSet<object>): any {
	if (
		value === null ||
		typeof value === "string" ||
		typeof value === "number" ||
		typeof value === "boolean"
	) {
		return value;
	}
	if (typeof value === "bigint") return value.toString();
	if (typeof value === "undefined" || typeof value === "function") return undefined;
	if (value instanceof Uint8Array) return value;
	if (value instanceof Date) return value.toISOString();
	if (value instanceof Error) {
		return { name: value.name, message: value.message, stack: value.stack };
	}
	if (typeof value !== "object") return String(value);
	if (seen.has(value)) return "[Circular]";
	seen.add(value);
	try {
		if (Array.isArray(value)) {
			return value.map((item) => sanitize(item, seen));
		}
		return Object.fromEntries(
			Object.entries(value)
				.filter(([, item]) => typeof item !== "function")
				.map(([key, item]) => [key, sanitize(item, seen)]),
		);
	} finally {
		seen.delete(value);
	}
}
