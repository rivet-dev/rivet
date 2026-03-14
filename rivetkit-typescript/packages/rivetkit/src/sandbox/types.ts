import type { ActionContext } from "@/actor/contexts";
import type { DatabaseProvider } from "@/actor/database";
import type { RawAccess } from "@/db/config";
import type {
	SandboxAgent,
	SandboxAgentConnectOptions,
	SessionPersistDriver,
} from "sandbox-agent";

// Keep this split in lockstep with the sandbox-agent SDK. Hooks should match the
// SDK callback methods, and actions should match every other SDK instance
// method. Update these lists and the parity test together when sandbox-agent
// changes.
export const SANDBOX_AGENT_HOOK_METHODS = [
	"onSessionEvent",
	"onPermissionRequest",
] as const;

export type SandboxActorHookMethodName =
	(typeof SANDBOX_AGENT_HOOK_METHODS)[number];

export const SANDBOX_AGENT_ACTION_METHODS = [
	"dispose",
	"listSessions",
	"getSession",
	"getEvents",
	"createSession",
	"resumeSession",
	"resumeOrCreateSession",
	"destroySession",
	"setSessionMode",
	"setSessionConfigOption",
	"setSessionModel",
	"setSessionThoughtLevel",
	"getSessionConfigOptions",
	"getSessionModes",
	"rawSendSessionMethod",
	"respondPermission",
	"rawRespondPermission",
	"getHealth",
	"listAgents",
	"getAgent",
	"installAgent",
	"listAcpServers",
	"listFsEntries",
	"readFsFile",
	"writeFsFile",
	"deleteFsEntry",
	"mkdirFs",
	"moveFs",
	"statFs",
	"uploadFsBatch",
	"getMcpConfig",
	"setMcpConfig",
	"deleteMcpConfig",
	"getSkillsConfig",
	"setSkillsConfig",
	"deleteSkillsConfig",
	"getProcessConfig",
	"setProcessConfig",
	"createProcess",
	"runProcess",
	"listProcesses",
	"getProcess",
	"stopProcess",
	"killProcess",
	"deleteProcess",
	"getProcessLogs",
	"followProcessLogs",
	"sendProcessInput",
	"resizeProcessTerminal",
	"buildProcessTerminalWebSocketUrl",
	"connectProcessTerminalWebSocket",
	"connectProcessTerminal",
] as const;

export type SandboxAgentActionMethodName =
	(typeof SANDBOX_AGENT_ACTION_METHODS)[number];

export type SandboxActorActions = Pick<
	SandboxAgent,
	SandboxAgentActionMethodName
>;

export type SandboxSessionEvent = Parameters<
	Parameters<SandboxAgent["onSessionEvent"]>[1]
>[0];

export interface SandboxActorState {
	sandboxId: string | null;
	providerName: string | null;
	/** Persisted so that on wake, the actor knows which sessions to
	 * re-subscribe to when reconnecting to the sandbox agent. Without
	 * this, event listeners would be lost after a sleep/wake cycle. */
	subscribedSessionIds: string[];
	sandboxDestroyed: boolean;
}

export interface SandboxActorProviderCreateContext {
	actorId: string;
	actorKey: readonly string[];
}

export interface SandboxActorProviderConnectOptions
	extends Pick<
		SandboxAgentConnectOptions,
		"headers" | "replayMaxEvents" | "replayMaxChars" | "waitForHealth"
	> {
	persist: SessionPersistDriver;
}

export interface SandboxActorProvider {
	name: string;
	create(context: SandboxActorProviderCreateContext): Promise<string>;
	destroy(sandboxId: string): Promise<void>;
	connectAgent(
		sandboxId: string,
		options: SandboxActorProviderConnectOptions,
	): Promise<SandboxAgent>;
	/**
	 * Optional hook called before `connectAgent` to ensure the sandbox
	 * environment is running. Providers like Daytona use this to restart
	 * the sandbox-agent process after the sandbox sleeps or restarts.
	 */
	wake?(sandboxId: string): Promise<void>;
}

export interface SandboxActorVars {
	sandboxAgentClient: SandboxAgent | null;
	provider: SandboxActorProvider | null;
	activeSessionIds: Set<string>;
	activePromptRequestIdsBySessionId: Map<string, string[]>;
	lastEventAtBySessionId: Map<string, number>;
	unsubscribeBySessionId: Map<
		string,
		{
			event?: () => void;
			permission?: () => void;
		}
	>;
	/** Tracks in-flight hook promises. Size is used instead of a counter
	 * to avoid increment/decrement mismatch bugs. */
	activeHooks: Set<Promise<void>>;
	warningTimeoutBySessionId: Map<string, ReturnType<typeof setTimeout>>;
	staleTimeoutBySessionId: Map<string, ReturnType<typeof setTimeout>>;
}

/** @deprecated Use `SandboxActorVars` instead. */
export type SandboxActorRuntime = SandboxActorVars;

/**
 * Action context type used by the sandbox actor implementation for session
 * management, proxy actions, and lifecycle hooks.
 */
export type SandboxActionContext<TConnParams = undefined> = ActionContext<
	SandboxActorState,
	TConnParams,
	undefined,
	SandboxActorVars,
	undefined,
	DatabaseProvider<RawAccess>
>;
