// @rivet-dev/agent-os

export {
	createInMemoryFileSystem,
	KernelError,
} from "@secure-exec/core";
export type {
	NetworkAccessRequest,
	OpenShellOptions,
	PermissionCheck,
	PermissionDecision,
	Permissions,
	ProcessInfo,
	VirtualDirEntry,
	VirtualFileSystem,
	VirtualStat,
} from "@secure-exec/core";
export type { NotificationHandler } from "./acp-client.js";
export { AcpClient } from "./acp-client.js";
export type {
	AgentOsContext,
	AgentOsOptions,
	AgentRegistryEntry,
	BatchReadResult,
	BatchWriteEntry,
	BatchWriteResult,
	CreateSessionOptions,
	DirEntry,
	McpServerConfig,
	McpServerConfigLocal,
	McpServerConfigRemote,
	MountConfig,
	ProcessTreeNode,
	ReaddirRecursiveOptions,
	SessionInfo,
	SpawnedProcessInfo,
} from "./agent-os.js";
export { AgentOs } from "./agent-os.js";
export type {
	AgentConfig,
	AgentType,
	PrepareInstructionsOptions,
} from "./agents.js";
export { AGENT_CONFIGS } from "./agents.js";
export type {
	AgentSoftwareDescriptor,
	AnySoftwareDescriptor,
	SoftwareContext,
	SoftwareDescriptor,
	SoftwareInput,
	SoftwareRoot,
	ToolSoftwareDescriptor,
	WasmCommandDirDescriptor,
	WasmCommandSoftwareDescriptor,
} from "./packages.js";
export { defineSoftware } from "./packages.js";
export type { HostDirBackendOptions } from "./backends/host-dir-backend.js";
export { createHostDirBackend } from "./backends/host-dir-backend.js";
export type { OverlayBackendOptions } from "./backends/overlay-backend.js";
export { createOverlayBackend } from "./backends/overlay-backend.js";
export type {
	CronAction,
	CronEvent,
	CronEventHandler,
	CronJob,
	CronJobInfo,
	CronJobOptions,
	ScheduleDriver,
	ScheduleEntry,
	ScheduleHandle,
} from "./cron/index.js";
export { CronManager, TimerScheduleDriver } from "./cron/index.js";
export type { HostTool, ToolExample, ToolKit } from "./host-tools.js";
export { hostTool, toolKit, validateToolkits, MAX_TOOL_DESCRIPTION_LENGTH } from "./host-tools.js";
export { generateToolReference } from "./host-tools-prompt.js";
export {
	camelToKebab,
	getZodDescription,
	getZodEnumValues,
	parseArgv,
} from "./host-tools-argv.js";
export type { FieldInfo } from "./host-tools-argv.js";
export {
	createShimFilesystem,
	generateMasterShim,
	generateToolkitShim,
} from "./host-tools-shims.js";
export { getOsInstructions } from "./os-instructions.js";
export type {
	JsonRpcError,
	JsonRpcNotification,
	JsonRpcRequest,
	JsonRpcResponse,
} from "./protocol.js";
export {
	deserializeMessage,
	isResponse,
	serializeMessage,
} from "./protocol.js";
export type {
	AgentCapabilities,
	AgentInfo,
	GetEventsOptions,
	PermissionReply,
	PermissionRequest,
	PermissionRequestHandler,
	SequencedEvent,
	Session,
	SessionConfigOption,
	SessionEventHandler,
	SessionInitData,
	SessionMode,
	SessionModeState,
} from "./session.js";
export { createStdoutLineIterable } from "./stdout-lines.js";
