import type {
	AgentCapabilities,
	AgentInfo,
	AgentOs,
	CronEvent,
	JsonRpcNotification,
	JsonRpcResponse,
	PermissionRequest,
} from "@rivet-dev/agent-os-core";
import type { ActionContext } from "@/actor/contexts";
import type { DatabaseProvider } from "@/actor/database";
import type { RawAccess } from "@/db/config";

// --- Actor state (persisted across sleep/wake) ---

export interface AgentOsActorState {
	/** Sandbox ID persisted across sleep/wake so the `createOptions`
	 * callback can reconnect to the same sandbox instead of provisioning
	 * a new one. Format is `"{provider}/{rawId}"` (e.g. `"docker/abc123"`).
	 * Set by the user inside `createOptions`; read back on subsequent wakes. */
	sandboxId: string | null;
}

// --- Actor vars (ephemeral, recreated on wake) ---

export interface AgentOsActorVars {
	agentOs: AgentOs | null;
	/** In-flight VM boot promise used to prevent concurrent ensureVm calls from
	 * creating duplicate VMs. Reset on sleep/wake and cleared on boot failure. */
	vmBootPromise: Promise<AgentOs> | null;
	activeSessionIds: Set<string>;
	activeProcesses: Set<number>;
	activeHooks: Set<Promise<void>>;
	activeShells: Set<string>;
	sessions: Set<string>;
}

// --- Event payloads ---

export interface SessionEventPayload {
	sessionId: string;
	event: JsonRpcNotification;
}

export interface PermissionRequestPayload {
	sessionId: string;
	request: PermissionRequest;
}

export type VmBootedPayload = Record<string, never>;

export interface VmShutdownPayload {
	reason: "sleep" | "destroy" | "error";
}

export interface ProcessOutputPayload {
	pid: number;
	stream: "stdout" | "stderr";
	data: Uint8Array;
}

export interface ProcessExitPayload {
	pid: number;
	exitCode: number;
}

export interface ShellDataPayload {
	shellId: string;
	data: Uint8Array;
}

export interface CronEventPayload {
	event: CronEvent;
}

// --- Event schema map (used by actor() events config) ---

export interface AgentOsEvents {
	sessionEvent: SessionEventPayload;
	permissionRequest: PermissionRequestPayload;
	vmBooted: VmBootedPayload;
	vmShutdown: VmShutdownPayload;
	processOutput: ProcessOutputPayload;
	processExit: ProcessExitPayload;
	shellData: ShellDataPayload;
	cronEvent: CronEventPayload;
}

// --- Prompt result ---

/** Result from sendPrompt. */
export interface PromptResult {
	/** Raw JSON-RPC response from the ACP adapter. */
	response: JsonRpcResponse;
	/** Accumulated agent text output from streamed message chunks. */
	text: string;
}

// --- Session serialization ---

export interface SessionRecord {
	sessionId: string;
	agentType: string;
	capabilities: AgentCapabilities;
	agentInfo: AgentInfo | null;
}

// --- Persisted session types ---

export interface PersistedSessionRecord {
	sessionId: string;
	agentType: string;
	capabilities: AgentCapabilities;
	agentInfo: AgentInfo | null;
	createdAt: number;
}

export interface PersistedSessionEvent {
	sessionId: string;
	seq: number;
	event: JsonRpcNotification;
	createdAt: number;
}

// --- Serializable cron action (excludes callback type) ---

export type SerializableCronAction =
	| { type: "session"; agentType: string; prompt: string; cwd?: string }
	| { type: "exec"; command: string; args?: string[] };

export interface SerializableCronJobOptions {
	id?: string;
	schedule: string;
	action: SerializableCronAction;
	overlap?: "allow" | "skip" | "queue";
}

// --- Action context alias ---

export type AgentOsActionContext<TConnParams = undefined> = ActionContext<
	AgentOsActorState,
	TConnParams,
	undefined,
	AgentOsActorVars,
	undefined,
	DatabaseProvider<RawAccess>
>;
