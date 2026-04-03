// Database migration

// Cron actions
export { buildCronActions } from "./actor/cron";
export { migrateAgentOsTables } from "./actor/db";
// Filesystem actions
export { buildFilesystemActions } from "./actor/filesystem";
// Actor factory and VM lifecycle helpers
export { agentOs, ensureVm, runHook, syncPreventSleep } from "./actor/index";
// Network actions
export {
	buildNetworkActions,
	type VmFetchOptions,
	type VmFetchResult,
} from "./actor/network";
// Preview actions
export {
	buildOnRequestHandler,
	buildPreviewActions,
	generateToken,
} from "./actor/preview";
// Process actions
export { buildProcessActions } from "./actor/process";
// Session actions
export {
	buildConfigActions,
	buildPromptActions,
	buildSessionActions,
	buildSessionPersistenceActions,
	subscribeToSession,
} from "./actor/session";
// Shell actions
export { buildShellActions } from "./actor/shell";
// User-facing alias for the createOptions callback context parameter.
export type { AgentOsActorContext as AgentOsCreateContext } from "./config";
// Config schema and types
export {
	type AgentOsActorConfig,
	type AgentOsActorConfigInput,
	type AgentOsActorContext,
	agentOsActorConfigSchema,
} from "./config";
// SQLite-backed VFS
export { createSqliteVfs, type SqliteVfsOptions } from "./fs/sqlite-vfs";
// Domain types and event payloads
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
