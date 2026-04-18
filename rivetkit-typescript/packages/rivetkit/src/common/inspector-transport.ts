import type { WorkflowHistory } from "@/common/bare/transport/v1";
import {
	decodeWorkflowHistory,
	encodeWorkflowHistory,
} from "@/common/bare/transport/v1";
import { bufferToArrayBuffer, toUint8Array } from "@/utils";

export function encodeWorkflowHistoryTransport(
	history: WorkflowHistory,
): ArrayBuffer {
	return bufferToArrayBuffer(encodeWorkflowHistory(history));
}

export function decodeWorkflowHistoryTransport(
	data: ArrayBuffer | ArrayBufferView,
): WorkflowHistory {
	return decodeWorkflowHistory(toUint8Array(data));
}
