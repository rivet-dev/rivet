// Data contracts for the agentOS actor inspector.
//
// These model the eventual "manifest" inspector RPC + per-tab data so the live
// implementation (see `use-agent-os-inspector.tsx`'s live stub) can reuse the
// exact same shapes. The prototype feeds them from `fixtures.ts`.
//
// agentOS is a portable, lightweight operating system for agents that runs in
// your process (Wasm + V8 isolates) — not an isolated VM or heavyweight sandbox.

export type AgentType = "pi" | "amp" | "claude-code" | "codex" | "opencode";

export interface AgentInfo {
	type: AgentType;
	/** Semver, e.g. "0.3.1". */
	version: string;
}

export type SoftwareSource = "rivet-dev" | "user";

export interface SoftwareBundle {
	/** npm package name, e.g. "@rivet-dev/agent-os-common". */
	name: string;
	version: string;
	source: SoftwareSource;
	/** CLI commands this bundle ships into the VM. */
	binaries: string[];
}

export interface ToolArg {
	name: string;
	/** Zod-derived type label, e.g. "string" | "number" | "boolean". */
	type: string;
	optional?: boolean;
}

export interface ToolDef {
	/** Tool name within its toolkit, e.g. "get". */
	tool: string;
	/** CLI form the agent invokes, e.g. "agentos-weather get". */
	command: string;
	description: string;
	args: ToolArg[];
}

export interface Toolkit {
	/** Toolkit name, e.g. "weather". */
	name: string;
	description?: string;
	tools: ToolDef[];
}

export interface Invocation {
	/** "{toolkit}.{tool}", e.g. "weather.get". */
	tool: string;
	input: unknown;
	output: unknown;
	latencyMs: number;
	/** Epoch ms. */
	at: number;
	/** Tool error code when the call failed, e.g. "weather.unknown_city". */
	error?: string;
}

export type MountKind = "persistent" | "s3" | "sandbox" | "gdrive";
export type MountStatus = "online" | "degraded";

export interface MountInfo {
	path: string;
	kind: MountKind;
	/** Backend identity, e.g. "s3://rivet-uploads", "e2b · sjc1", "team-drive/agents". */
	provider: string;
	sizeBytes?: number;
	status: MountStatus;
}

/** Filesystem nodes can sit under any mount kind, including ephemeral ones. */
export type FsMount = MountKind | "memory" | "host";

export interface FsNode {
	name: string;
	path: string;
	type: "dir" | "file";
	mount: FsMount;
	sizeBytes?: number;
	mtimeMs?: number;
	children?: FsNode[];
}

export type ProcessStatus = "running" | "sleeping" | "exited";

export interface ProcessInfo {
	pid: number;
	ppid: number;
	command: string;
	startedAt: number;
	/** CPU usage percent. */
	cpu?: number;
	/** Resident memory in bytes. */
	memBytes?: number;
	status: ProcessStatus;
	exitCode?: number;
	signal?: string;
	stdoutTail?: string;
}

export type ConnOrigin = "browser" | "cli";

export interface ConnInfo {
	connId: string;
	/** Stream label, e.g. "sessionEvent · ses_…" or "shell · pid 27". */
	stream: string;
	connectedAt: number;
	origin: ConnOrigin;
}

export type SessionStatus = "running" | "idle" | "error";

export interface SessionSummary {
	sessionId: string;
	agentType: AgentType;
	createdAt: number;
	eventCount: number;
	status: SessionStatus;
}

/**
 * Typed rendering model for the transcript stream. The live source maps ACP
 * `session/update` notifications into these events (real ACP method strings are
 * TBD and verified at wiring time). The prototype uses this union directly.
 */
export type TranscriptEvent =
	| { kind: "user"; seq: number; at: number; text: string }
	| { kind: "assistant"; seq: number; at: number; text: string }
	| { kind: "thinking"; seq: number; at: number; text: string }
	| {
			kind: "shell";
			seq: number;
			at: number;
			command: string;
			exitCode: number;
			durationMs: number;
			output: string;
	  }
	| {
			kind: "tool";
			seq: number;
			at: number;
			tool: string;
			input: unknown;
			output: unknown;
			latencyMs: number;
			error?: string;
	  }
	| {
			kind: "file_write";
			seq: number;
			at: number;
			path: string;
			bytes: number;
	  };

export interface AgentOsMetadata {
	actorId: string;
	actorKey: string;
	actorName: string;
	actorKind: "agent-os";
	/** Runner selector, e.g. "default · sjc1". */
	runner: string;
	region: string;
	/** Agent label, e.g. "pi 0.3.1". */
	agentVersion: string;
	/** @rivet-dev/agent-os-core version, e.g. "0.4.2". */
	agentosCore: string;
	/** Short software summary, e.g. ["common", "pi", "weather-toolkit"]. */
	software: string[];
	createdAt: number;
	lastSleepAt?: number;
	sessionCount: number;
	activeSessionCount: number;
}

/**
 * Identity + static config returned by the (future) agentOS manifest inspector
 * RPC. Its presence is also the signal that an actor is an agentOS actor.
 */
export interface AgentOsManifest {
	kind: "agent-os";
	agent: AgentInfo;
	agentosCore: string;
	software: SoftwareBundle[];
	toolkits: Toolkit[];
	mounts: MountInfo[];
	metadata: AgentOsMetadata;
}

export interface ToolsData {
	toolkits: Toolkit[];
	invocations: Invocation[];
}

export interface FileContent {
	path: string;
	sizeBytes: number;
	mtimeMs: number;
	/** UTF-8 text content, or null for binary files. */
	text: string | null;
	/** Language hint for the viewer, e.g. "json", "python", "markdown". */
	language?: string;
}
