import type { WorkflowHistory } from "@/schemas/transport/mod";
import {
	decodeWorkflowHistory,
	encodeWorkflowHistory,
} from "@/schemas/transport/mod";
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
