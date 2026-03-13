import type { ActorConfigInput } from "@/actor/config";
import type {
	ActorContext,
	BeforeConnectContext,
	CreateContext,
} from "@/actor/contexts";
import type { DatabaseProvider } from "@/actor/database";
import type { AnyDatabaseProvider } from "@/actor/database";
import type { RawAccess } from "@/db/config";
import type {
	PermissionRequestListener,
	SandboxAgent,
	SandboxAgentConnectOptions,
	SessionEventListener,
	SessionPersistDriver,
} from "sandbox-agent";

export interface SandboxActorState {
	sandboxId: string | null;
	sessionIds: string[];
	providerName: string | null;
}

export interface SandboxActorRuntime {
	agent: SandboxAgent | null;
	provider: SandboxActorProvider | null;
	unsubscribeBySessionId: Map<
		string,
		{
			event?: () => void;
			permission?: () => void;
		}
	>;
}

export type SandboxActorHookMethodName =
	| "onSessionEvent"
	| "onPermissionRequest";

// Keep this split in lockstep with the sandbox-agent SDK. Hooks should match the
// SDK callback methods, and actions should match every other SDK instance
// method. Update these lists and the parity test together when sandbox-agent
// changes.
export const SANDBOX_AGENT_HOOK_METHODS = [
	"onSessionEvent",
	"onPermissionRequest",
] as const satisfies readonly SandboxActorHookMethodName[];

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

export type SandboxSessionPermissionRequest = Parameters<
	Parameters<SandboxAgent["onPermissionRequest"]>[1]
>[0];

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
}

export type SandboxActorHookContext<
	TConnParams = undefined,
	TInput = undefined,
> = ActorContext<
	SandboxActorState,
	TConnParams,
	undefined,
	SandboxActorRuntime,
	TInput,
	AnyDatabaseProvider
>;

export type SandboxActorCreateProviderContext<TInput = undefined> =
	CreateContext<SandboxActorState, TInput, AnyDatabaseProvider>;

export type SandboxActorOnBeforeConnect<
	TConnParams = undefined,
	TInput = undefined,
> =
	ActorConfigInput<
		SandboxActorState,
		TConnParams,
		undefined,
		SandboxActorRuntime,
		TInput,
		AnyDatabaseProvider
	>["onBeforeConnect"];

export type SandboxActorCreateProvider<TInput = undefined> = (
	c: SandboxActorCreateProviderContext<TInput>,
	input: TInput,
) => SandboxActorProvider | Promise<SandboxActorProvider>;

type SandboxActorConfigBase<TConnParams, TInput> = {
	database: DatabaseProvider<RawAccess>;
	persistRawEvents?: boolean;
	onBeforeConnect?: SandboxActorOnBeforeConnect<TConnParams, TInput>;
	onSessionEvent?: (
		c: SandboxActorHookContext<TConnParams, TInput>,
		sessionId: string,
		event: Parameters<SessionEventListener>[0],
	) => void | Promise<void>;
	onPermissionRequest?: (
		c: SandboxActorHookContext<TConnParams, TInput>,
		sessionId: string,
		request: Parameters<PermissionRequestListener>[0],
	) => void | Promise<void>;
};

export type SandboxActorConfig<
	TConnParams = undefined,
	TInput = undefined,
> = SandboxActorConfigBase<TConnParams, TInput> &
	(
		| {
				provider: SandboxActorProvider;
				createProvider?: never;
		  }
		| {
				provider?: never;
				createProvider: SandboxActorCreateProvider<TInput>;
		  }
	);

export type SandboxActorBeforeConnectContext<
	TConnParams = undefined,
	TInput = undefined,
> =
	BeforeConnectContext<
		SandboxActorState,
		SandboxActorRuntime,
		TInput,
		AnyDatabaseProvider
	>;
