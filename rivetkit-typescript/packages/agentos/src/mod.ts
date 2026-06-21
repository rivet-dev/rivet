/**
 * `@rivet-dev/agentos` — the user-facing Agent OS integration for RivetKit.
 *
 * This package is the standalone home of the integration that historically
 * shipped as the `rivetkit/agent-os` subpath export. It re-exports that
 * surface verbatim so behavior is identical and the underlying Rust path is
 * unchanged: the implementation still lives in `rivetkit` and continues to
 * call into the published `@rivet-dev/agent-os-*` packages (sidecar, core,
 * pi, etc.) exactly as before.
 */

export {
	agentOs,
	type AgentOsActorDefinition,
	nodeModulesMount,
	type NodeModulesMountConfig,
	type AgentOsActorConfig,
	type AgentOsActorConfigInput,
	agentOsActorConfigSchema,
	type AgentOsActionContext,
	type AgentOsActorState,
	type AgentOsActorVars,
	type AgentOsEvents,
	type CronEventPayload,
	type PermissionRequestPayload,
	type PersistedSessionEvent,
	type PersistedSessionRecord,
	type ProcessExitPayload,
	type ProcessOutputPayload,
	type PromptResult,
	type SerializableCronAction,
	type SerializableCronJobOptions,
	type SessionEventPayload,
	type SessionRecord,
	type ShellDataPayload,
	type VmBootedPayload,
	type VmShutdownPayload,
} from "rivetkit/agent-os";
