import type {
	AgentSession,
	PromptOptions,
	SessionManager,
} from "@earendil-works/pi-coding-agent";
import {
	ensurePiSession,
	persistPiState,
	type PiContext,
	type PiSession,
	type PiSessionOptions,
} from "./runtime.js";

/** An action that mirrors one `AgentSession` method: same arguments, same result. */
type SessionMethodAction<K extends keyof AgentSession> =
	AgentSession[K] extends (...args: infer TArgs) => infer TResult
		? (c: PiContext, ...args: TArgs) => Promise<Awaited<TResult>>
		: never;

export type PiPromptOptions = Omit<PromptOptions, "preflightResult">;

export type PiBashResult = Awaited<ReturnType<AgentSession["executeBash"]>>;

export interface PiSessionInfo {
	sessionId: string;
	sessionName: string | undefined;
	cwd: string;
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

export interface PiActions {
	/**
	 * Sends a prompt and resolves when Pi's run ends, after retries and tool
	 * calls. Progress streams through the `event` event. A model error does not
	 * reject: the last assistant message has `stopReason: "error"`. A prompt
	 * queued with `streamingBehavior` resolves once it is queued.
	 */
	prompt: (
		c: PiContext,
		text: string,
		options?: PiPromptOptions,
	) => Promise<void>;
	steer: SessionMethodAction<"steer">;
	followUp: SessionMethodAction<"followUp">;
	clearQueue: SessionMethodAction<"clearQueue">;
	abort: SessionMethodAction<"abort">;
	waitForIdle: SessionMethodAction<"waitForIdle">;

	setThinkingLevel: SessionMethodAction<"setThinkingLevel">;
	cycleThinkingLevel: SessionMethodAction<"cycleThinkingLevel">;
	getAvailableThinkingLevels: SessionMethodAction<"getAvailableThinkingLevels">;
	supportsThinking: SessionMethodAction<"supportsThinking">;

	getActiveToolNames: SessionMethodAction<"getActiveToolNames">;
	getAllTools: SessionMethodAction<"getAllTools">;
	setActiveToolsByName: SessionMethodAction<"setActiveToolsByName">;
	executeBash: (
		c: PiContext,
		command: string,
		options?: { excludeFromContext?: boolean; id?: string },
	) => Promise<PiBashResult>;
	abortBash: SessionMethodAction<"abortBash">;

	setSteeringMode: SessionMethodAction<"setSteeringMode">;
	setFollowUpMode: SessionMethodAction<"setFollowUpMode">;
	compact: SessionMethodAction<"compact">;
	abortCompaction: SessionMethodAction<"abortCompaction">;
	setAutoCompactionEnabled: SessionMethodAction<"setAutoCompactionEnabled">;
	abortRetry: SessionMethodAction<"abortRetry">;
	setAutoRetryEnabled: SessionMethodAction<"setAutoRetryEnabled">;

	setSessionName: SessionMethodAction<"setSessionName">;
	navigateTree: SessionMethodAction<"navigateTree">;
	getSessionTree: (
		c: PiContext,
	) => Promise<ReturnType<SessionManager["getTree"]>>;

	getSession: (c: PiContext) => Promise<PiSessionInfo>;
	getMessages: (c: PiContext) => Promise<AgentSession["messages"]>;
	getSteeringMessages: SessionMethodAction<"getSteeringMessages">;
	getFollowUpMessages: SessionMethodAction<"getFollowUpMessages">;
	getSessionStats: SessionMethodAction<"getSessionStats">;
	getContextUsage: SessionMethodAction<"getContextUsage">;
	getLastAssistantText: SessionMethodAction<"getLastAssistantText">;
}

export function createPiActions(options: PiSessionOptions): PiActions {
	/** Runs a read-only session method. */
	const read = async <T>(
		c: PiContext,
		callback: (handle: PiSession) => T | Promise<T>,
	): Promise<T> => callback(await ensurePiSession(c, options));

	/** Runs a session method that may change settings and persists them after. */
	const mutate = async <T>(
		c: PiContext,
		callback: (handle: PiSession) => T | Promise<T>,
	): Promise<T> => {
		const handle = await ensurePiSession(c, options);
		let result: T;
		try {
			result = await callback(handle);
		} catch (error) {
			await persistPiState(c, handle).catch((writeError: unknown) => {
				c.log.error({ msg: "pi state write failed after an action error", error: writeError });
			});
			throw error;
		}
		await persistPiState(c, handle);
		return result;
	};

	return {
		prompt: (c, text, promptOptions) =>
			mutate(c, async ({ session }) => {
				const abort = () => void session.abort();
				c.abortSignal.addEventListener("abort", abort, { once: true });
				try {
					await session.prompt(text, promptOptions);
				} finally {
					c.abortSignal.removeEventListener("abort", abort);
				}
			}),
		steer: (c, ...args) => mutate(c, ({ session }) => session.steer(...args)),
		followUp: (c, ...args) =>
			mutate(c, ({ session }) => session.followUp(...args)),
		clearQueue: (c) => mutate(c, ({ session }) => session.clearQueue()),
		abort: (c) => mutate(c, ({ session }) => session.abort()),
		waitForIdle: (c) => read(c, ({ session }) => session.waitForIdle()),

		setThinkingLevel: (c, ...args) =>
			mutate(c, ({ session }) => session.setThinkingLevel(...args)),
		cycleThinkingLevel: (c, ...args) =>
			mutate(c, ({ session }) => session.cycleThinkingLevel(...args)),
		getAvailableThinkingLevels: (c) =>
			read(c, ({ session }) => session.getAvailableThinkingLevels()),
		supportsThinking: (c) =>
			read(c, ({ session }) => session.supportsThinking()),

		getActiveToolNames: (c) =>
			read(c, ({ session }) => session.getActiveToolNames()),
		getAllTools: (c) => read(c, ({ session }) => session.getAllTools()),
		setActiveToolsByName: (c, ...args) =>
			mutate(c, ({ session }) => session.setActiveToolsByName(...args)),
		executeBash: (c, command, bashOptions) =>
			mutate(c, ({ session, bashOperations }) =>
				session.executeBash(command, undefined, {
					...bashOptions,
					operations: bashOperations,
				}),
			),
		abortBash: (c) => read(c, ({ session }) => session.abortBash()),

		setSteeringMode: (c, ...args) =>
			mutate(c, ({ session }) => session.setSteeringMode(...args)),
		setFollowUpMode: (c, ...args) =>
			mutate(c, ({ session }) => session.setFollowUpMode(...args)),
		compact: (c, ...args) =>
			mutate(c, ({ session }) => session.compact(...args)),
		abortCompaction: (c) =>
			read(c, ({ session }) => session.abortCompaction()),
		setAutoCompactionEnabled: (c, ...args) =>
			mutate(c, ({ session }) => session.setAutoCompactionEnabled(...args)),
		abortRetry: (c) => read(c, ({ session }) => session.abortRetry()),
		setAutoRetryEnabled: (c, ...args) =>
			mutate(c, ({ session }) => session.setAutoRetryEnabled(...args)),

		setSessionName: (c, ...args) =>
			mutate(c, ({ session }) => session.setSessionName(...args)),
		navigateTree: (c, ...args) =>
			mutate(c, ({ session }) => session.navigateTree(...args)),
		getSessionTree: (c) =>
			read(c, ({ session }) => session.sessionManager.getTree()),

		getSession: (c) =>
			read(c, ({ session, cwd }) => ({
				sessionId: session.sessionId,
				sessionName: session.sessionName,
				cwd,
				model: session.model,
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
		getMessages: (c) => read(c, ({ session }) => session.messages),
		getSteeringMessages: (c) =>
			read(c, ({ session }) => [...session.getSteeringMessages()]),
		getFollowUpMessages: (c) =>
			read(c, ({ session }) => [...session.getFollowUpMessages()]),
		getSessionStats: (c) => read(c, ({ session }) => session.getSessionStats()),
		getContextUsage: (c) => read(c, ({ session }) => session.getContextUsage()),
		getLastAssistantText: (c) =>
			read(c, ({ session }) => session.getLastAssistantText()),
	};
}

