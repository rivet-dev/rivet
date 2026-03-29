import type {
	AgentCapabilities,
	AgentInfo,
	AgentOs,
	CronEvent,
	JsonRpcNotification,
	PermissionRequest,
} from "@rivet-dev/agent-os-core";
import type { ActionContext } from "@/actor/contexts";

// --- Actor state (persisted across sleep/wake) ---

// biome-ignore lint/complexity/noBannedTypes: empty state placeholder, consumers extend via generics
export type AgentOsActorState = {};

// --- Actor vars (ephemeral, recreated on wake) ---

export interface AgentOsActorVars {
	agentOs: AgentOs | null;
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
	any
>;
