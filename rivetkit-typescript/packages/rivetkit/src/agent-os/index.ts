// Rust-backed agent-os actor surface.
//
// Phase 1c: only the `agentOs()` definition function, the config schema,
// and the public domain types are re-exported. Legacy JS-port action
// builders (cron/db/filesystem/network/preview/process/session/shell)
// were removed along with the JS-port implementation files. Subsequent
// phases (3+) add new action arms to the Rust crate, not new TS modules.

export { agentOs, type AgentOsActorDefinition } from "./actor/index";

export {
	type AgentOsActorConfig,
	type AgentOsActorConfigInput,
	agentOsActorConfigSchema,
} from "./config";

export type {
	AgentOsActionContext,
	AgentOsActorState,
	AgentOsActorVars,
	AgentOsEvents,
	CronEventPayload,
	PermissionRequestPayload,
	PersistedSessionEvent,
	PersistedSessionRecord,
	ProcessExitPayload,
	ProcessOutputPayload,
	PromptResult,
	SerializableCronAction,
	SerializableCronJobOptions,
	SessionEventPayload,
	SessionRecord,
	ShellDataPayload,
	VmBootedPayload,
	VmShutdownPayload,
} from "./types";
