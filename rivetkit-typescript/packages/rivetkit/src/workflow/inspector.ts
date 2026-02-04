import * as cbor from "cbor-x";
import { createNanoEvents } from "nanoevents";
import type {
	BranchStatus,
	BranchStatusType,
	EntryKind,
	EntryStatus,
	Location,
	SleepState,
	WorkflowHistoryEntry,
	WorkflowHistorySnapshot,
	WorkflowEntryMetadataSnapshot,
} from "@rivetkit/workflow-engine";
import { encodeWorkflowHistoryTransport } from "@/inspector/transport";
import type * as inspectorSchema from "@/schemas/actor-inspector/mod";
import * as transport from "@/schemas/transport/mod";
import { assertUnreachable, bufferToArrayBuffer } from "@/utils";

export interface WorkflowInspectorAdapter {
	getHistory: () => inspectorSchema.WorkflowHistory | null;
	onHistoryUpdated: (
		listener: (history: inspectorSchema.WorkflowHistory) => void,
	) => () => void;
}

export function createWorkflowInspectorAdapter(): {
	adapter: WorkflowInspectorAdapter;
	update: (snapshot: WorkflowHistorySnapshot) => void;
} {
	const emitter = createNanoEvents<{
		updated: (history: inspectorSchema.WorkflowHistory) => void;
	}>();
	let history: inspectorSchema.WorkflowHistory | null = null;

	const adapter: WorkflowInspectorAdapter = {
		getHistory: () => history,
		onHistoryUpdated: (listener) => emitter.on("updated", listener),
	};

	const update = (snapshot: WorkflowHistorySnapshot) => {
		const transportHistory = toWorkflowHistory(snapshot);
		const next = encodeWorkflowHistoryTransport(transportHistory);
		history = next;
		emitter.emit("updated", next);
	};

	return { adapter, update };
}

function encodeCbor(value: unknown): ArrayBuffer {
	return bufferToArrayBuffer(cbor.encode(value));
}

function encodeOptionalCbor(value: unknown): ArrayBuffer | null {
	if (value === undefined) {
		return null;
	}
	return encodeCbor(value);
}

function toU64(value: number): bigint {
	return BigInt(Math.max(0, Math.floor(value)));
}

function toWorkflowLocation(
	location: Location,
): transport.WorkflowLocation {
	return location.map((segment) => {
		if (typeof segment === "number") {
			return { tag: "WorkflowNameIndex", val: segment };
		}
		return {
			tag: "WorkflowLoopIterationMarker",
			val: {
				loop: segment.loop,
				iteration: segment.iteration,
			},
		};
	});
}

function toWorkflowEntryKind(
	kind: EntryKind,
): transport.WorkflowEntryKind {
	switch (kind.type) {
		case "step":
			return {
				tag: "WorkflowStepEntry",
				val: {
					output: encodeOptionalCbor(kind.data.output),
					error: kind.data.error ?? null,
				},
			};
		case "loop":
			return {
				tag: "WorkflowLoopEntry",
				val: {
					state: encodeCbor(kind.data.state),
					iteration: kind.data.iteration,
					output: encodeOptionalCbor(kind.data.output),
				},
			};
		case "sleep":
			return {
				tag: "WorkflowSleepEntry",
				val: {
					deadline: toU64(kind.data.deadline),
					state: toWorkflowSleepState(kind.data.state),
				},
			};
		case "message":
			return {
				tag: "WorkflowMessageEntry",
				val: {
					name: kind.data.name,
					messageData: encodeCbor(kind.data.data),
				},
			};
		case "rollback_checkpoint":
			return {
				tag: "WorkflowRollbackCheckpointEntry",
				val: { name: kind.data.name },
			};
		case "join":
			return {
				tag: "WorkflowJoinEntry",
				val: { branches: toWorkflowBranchStatusMap(kind.data.branches) },
			};
		case "race":
			return {
				tag: "WorkflowRaceEntry",
				val: {
					winner: kind.data.winner ?? null,
					branches: toWorkflowBranchStatusMap(kind.data.branches),
				},
			};
		case "removed":
			return {
				tag: "WorkflowRemovedEntry",
				val: {
					originalType: kind.data.originalType,
					originalName: kind.data.originalName ?? null,
				},
			};
		default:
			assertUnreachable(kind as never);
	}
}

function toWorkflowEntry(
	entry: WorkflowHistoryEntry,
): transport.WorkflowEntry {
	return {
		id: entry.id,
		location: toWorkflowLocation(entry.location),
		kind: toWorkflowEntryKind(entry.kind),
	};
}

function toWorkflowEntryStatus(
	status: EntryStatus,
): transport.WorkflowEntryStatus {
	switch (status) {
		case "pending":
			return transport.WorkflowEntryStatus.PENDING;
		case "running":
			return transport.WorkflowEntryStatus.RUNNING;
		case "completed":
			return transport.WorkflowEntryStatus.COMPLETED;
		case "failed":
			return transport.WorkflowEntryStatus.FAILED;
		case "exhausted":
			return transport.WorkflowEntryStatus.EXHAUSTED;
		default:
			assertUnreachable(status as never);
	}
}

function toWorkflowSleepState(
	state: SleepState,
): transport.WorkflowSleepState {
	switch (state) {
		case "pending":
			return transport.WorkflowSleepState.PENDING;
		case "completed":
			return transport.WorkflowSleepState.COMPLETED;
		case "interrupted":
			return transport.WorkflowSleepState.INTERRUPTED;
		default:
			assertUnreachable(state as never);
	}
}

function toWorkflowBranchStatusType(
	status: BranchStatusType,
): transport.WorkflowBranchStatusType {
	switch (status) {
		case "pending":
			return transport.WorkflowBranchStatusType.PENDING;
		case "running":
			return transport.WorkflowBranchStatusType.RUNNING;
		case "completed":
			return transport.WorkflowBranchStatusType.COMPLETED;
		case "failed":
			return transport.WorkflowBranchStatusType.FAILED;
		case "cancelled":
			return transport.WorkflowBranchStatusType.CANCELLED;
		default:
			assertUnreachable(status as never);
	}
}

function toWorkflowBranchStatus(
	status: BranchStatus,
): transport.WorkflowBranchStatus {
	return {
		status: toWorkflowBranchStatusType(status.status),
		output: encodeOptionalCbor(status.output),
		error: status.error ?? null,
	};
}

function toWorkflowBranchStatusMap(
	branches: Record<string, BranchStatus>,
): ReadonlyMap<string, transport.WorkflowBranchStatus> {
	return new Map(
		Object.entries(branches).map(([name, status]) => [
			name,
			toWorkflowBranchStatus(status),
		]),
	);
}

function toWorkflowEntryMetadata(
	metadata: WorkflowEntryMetadataSnapshot,
): transport.WorkflowEntryMetadata {
	return {
		status: toWorkflowEntryStatus(metadata.status),
		error: metadata.error ?? null,
		attempts: metadata.attempts,
		lastAttemptAt: toU64(metadata.lastAttemptAt),
		createdAt: toU64(metadata.createdAt),
		completedAt:
			metadata.completedAt === undefined
				? null
				: toU64(metadata.completedAt),
		rollbackCompletedAt:
			metadata.rollbackCompletedAt === undefined
				? null
				: toU64(metadata.rollbackCompletedAt),
		rollbackError: metadata.rollbackError ?? null,
	};
}

function toWorkflowHistory(
	snapshot: WorkflowHistorySnapshot,
): transport.WorkflowHistory {
	const entryMetadata = new Map<string, transport.WorkflowEntryMetadata>();
	for (const [id, metadata] of snapshot.entryMetadata) {
		entryMetadata.set(id, toWorkflowEntryMetadata(metadata));
	}

	return {
		nameRegistry: snapshot.nameRegistry,
		entries: snapshot.entries.map((entry) => toWorkflowEntry(entry)),
		entryMetadata,
	};
}
