export * from "@/common/bare/generated/inspector/v4";
export {
	decodeWorkflowHistoryTransport,
	encodeWorkflowHistoryTransport,
} from "@/common/inspector-transport";
export {
	createWorkflowInspectorAdapter,
	type WorkflowInspectorAdapter,
} from "@/workflow/inspector";
export type {
	WorkflowHistory as TransportWorkflowHistory,
} from "@/common/bare/transport/v1";
