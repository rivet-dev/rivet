/**
 * Serialization/deserialization utilities for converting between
 * internal TypeScript types and BARE schema types.
 */

import * as cbor from "cbor-x";
import type * as v1 from "../dist/schemas/v1.js";
import {
	BranchStatusType as BareBranchStatusType,
	EntryStatus as BareEntryStatus,
	SleepState as BareSleepState,
} from "../dist/schemas/v1.js";
import type {
	BranchStatus as InternalBranchStatus,
	BranchStatusType as InternalBranchStatusType,
	Entry as InternalEntry,
	EntryKind as InternalEntryKind,
	EntryMetadata as InternalEntryMetadata,
	EntryStatus as InternalEntryStatus,
	Location as InternalLocation,
	LoopIterationMarker as InternalLoopIterationMarker,
	PathSegment as InternalPathSegment,
	SleepState as InternalSleepState,
	WorkflowState as InternalWorkflowState,
} from "../src/types.js";
import {
	CURRENT_VERSION,
	ENTRY_METADATA_VERSIONED,
	ENTRY_VERSIONED,
	WORKFLOW_METADATA_VERSIONED,
} from "./versioned.js";

// === Helper: ArrayBuffer to/from utilities ===

function bufferToArrayBuffer(buf: Uint8Array): ArrayBuffer {
	// Create a new ArrayBuffer and copy the data to ensure it's not a SharedArrayBuffer
	const arrayBuffer = new ArrayBuffer(buf.byteLength);
	new Uint8Array(arrayBuffer).set(buf);
	return arrayBuffer;
}

function encodeCbor(value: unknown): ArrayBuffer {
	return bufferToArrayBuffer(cbor.encode(value));
}

function decodeCbor<T>(data: ArrayBuffer): T {
	return cbor.decode(new Uint8Array(data)) as T;
}

/**
 * Validate that a value is a non-null object.
 */
function assertObject(
	value: unknown,
	context: string,
): asserts value is Record<string, unknown> {
	if (typeof value !== "object" || value === null) {
		throw new Error(`${context}: expected object, got ${typeof value}`);
	}
}

/**
 * Validate that a value is a string.
 */
function assertString(
	value: unknown,
	context: string,
): asserts value is string {
	if (typeof value !== "string") {
		throw new Error(`${context}: expected string, got ${typeof value}`);
	}
}

/**
 * Validate that a value is a number.
 */
function assertNumber(
	value: unknown,
	context: string,
): asserts value is number {
	if (typeof value !== "number") {
		throw new Error(`${context}: expected number, got ${typeof value}`);
	}
}

// === Entry Status Conversion ===

function entryStatusToBare(status: InternalEntryStatus): BareEntryStatus {
	switch (status) {
		case "pending":
			return BareEntryStatus.PENDING;
		case "running":
			return BareEntryStatus.RUNNING;
		case "completed":
			return BareEntryStatus.COMPLETED;
		case "failed":
			return BareEntryStatus.FAILED;
		case "exhausted":
			return BareEntryStatus.EXHAUSTED;
	}
}

function entryStatusFromBare(status: BareEntryStatus): InternalEntryStatus {
	switch (status) {
		case BareEntryStatus.PENDING:
			return "pending";
		case BareEntryStatus.RUNNING:
			return "running";
		case BareEntryStatus.COMPLETED:
			return "completed";
		case BareEntryStatus.FAILED:
			return "failed";
		case BareEntryStatus.EXHAUSTED:
			return "exhausted";
	}
}

// === Sleep State Conversion ===

function sleepStateToBare(state: InternalSleepState): BareSleepState {
	switch (state) {
		case "pending":
			return BareSleepState.PENDING;
		case "completed":
			return BareSleepState.COMPLETED;
		case "interrupted":
			return BareSleepState.INTERRUPTED;
	}
}

function sleepStateFromBare(state: BareSleepState): InternalSleepState {
	switch (state) {
		case BareSleepState.PENDING:
			return "pending";
		case BareSleepState.COMPLETED:
			return "completed";
		case BareSleepState.INTERRUPTED:
			return "interrupted";
	}
}

// === Branch Status Type Conversion ===

function branchStatusTypeToBare(
	status: InternalBranchStatusType,
): BareBranchStatusType {
	switch (status) {
		case "pending":
			return BareBranchStatusType.PENDING;
		case "running":
			return BareBranchStatusType.RUNNING;
		case "completed":
			return BareBranchStatusType.COMPLETED;
		case "failed":
			return BareBranchStatusType.FAILED;
		case "cancelled":
			return BareBranchStatusType.CANCELLED;
	}
}

function branchStatusTypeFromBare(
	status: BareBranchStatusType,
): InternalBranchStatusType {
	switch (status) {
		case BareBranchStatusType.PENDING:
			return "pending";
		case BareBranchStatusType.RUNNING:
			return "running";
		case BareBranchStatusType.COMPLETED:
			return "completed";
		case BareBranchStatusType.FAILED:
			return "failed";
		case BareBranchStatusType.CANCELLED:
			return "cancelled";
	}
}

// === Location Conversion ===

function locationToBare(location: InternalLocation): v1.Location {
	return location.map((segment): v1.PathSegment => {
		if (typeof segment === "number") {
			return { tag: "NameIndex", val: segment };
		}
		return {
			tag: "LoopIterationMarker",
			val: {
				loop: segment.loop,
				iteration: segment.iteration,
			},
		};
	});
}

function locationFromBare(location: v1.Location): InternalLocation {
	return location.map((segment): InternalPathSegment => {
		if (segment.tag === "NameIndex") {
			return segment.val;
		}
		return {
			loop: segment.val.loop,
			iteration: segment.val.iteration,
		};
	});
}

// === Branch Status Conversion ===

function branchStatusToBare(status: InternalBranchStatus): v1.BranchStatus {
	return {
		status: branchStatusTypeToBare(status.status),
		output: status.output !== undefined ? encodeCbor(status.output) : null,
		error: status.error ?? null,
	};
}

function branchStatusFromBare(status: v1.BranchStatus): InternalBranchStatus {
	return {
		status: branchStatusTypeFromBare(status.status),
		output: status.output !== null ? decodeCbor(status.output) : undefined,
		error: status.error ?? undefined,
	};
}

// === Entry Kind Conversion ===

function entryKindToBare(kind: InternalEntryKind): v1.EntryKind {
	switch (kind.type) {
		case "step":
			return {
				tag: "StepEntry",
				val: {
					output:
						kind.data.output !== undefined
							? encodeCbor(kind.data.output)
							: null,
					error: kind.data.error ?? null,
				},
			};
		case "loop":
			return {
				tag: "LoopEntry",
				val: {
					state: encodeCbor(kind.data.state),
					iteration: kind.data.iteration,
					output:
						kind.data.output !== undefined
							? encodeCbor(kind.data.output)
							: null,
				},
			};
		case "sleep":
			return {
				tag: "SleepEntry",
				val: {
					deadline: BigInt(kind.data.deadline),
					state: sleepStateToBare(kind.data.state),
				},
			};
		case "message":
			return {
				tag: "MessageEntry",
				val: {
					name: kind.data.name,
					messageData: encodeCbor(kind.data.data),
				},
			};
		case "rollback_checkpoint":
			return {
				tag: "RollbackCheckpointEntry",
				val: {
					name: kind.data.name,
				},
			};
		case "join":
			return {
				tag: "JoinEntry",
				val: {
					branches: new Map(
						Object.entries(kind.data.branches).map(
							([name, status]) => [
								name,
								branchStatusToBare(status),
							],
						),
					),
				},
			};
		case "race":
			return {
				tag: "RaceEntry",
				val: {
					winner: kind.data.winner,
					branches: new Map(
						Object.entries(kind.data.branches).map(
							([name, status]) => [
								name,
								branchStatusToBare(status),
							],
						),
					),
				},
			};
		case "removed":
			return {
				tag: "RemovedEntry",
				val: {
					originalType: kind.data.originalType,
					originalName: kind.data.originalName ?? null,
				},
			};
	}
}

function entryKindFromBare(kind: v1.EntryKind): InternalEntryKind {
	switch (kind.tag) {
		case "StepEntry":
			return {
				type: "step",
				data: {
					output:
						kind.val.output !== null
							? decodeCbor(kind.val.output)
							: undefined,
					error: kind.val.error ?? undefined,
				},
			};
		case "LoopEntry":
			return {
				type: "loop",
				data: {
					state: decodeCbor(kind.val.state),
					iteration: kind.val.iteration,
					output:
						kind.val.output !== null
							? decodeCbor(kind.val.output)
							: undefined,
				},
			};
		case "SleepEntry":
			return {
				type: "sleep",
				data: {
					deadline: Number(kind.val.deadline),
					state: sleepStateFromBare(kind.val.state),
				},
			};
		case "MessageEntry":
			return {
				type: "message",
				data: {
					name: kind.val.name,
					data: decodeCbor(kind.val.messageData),
				},
			};
		case "RollbackCheckpointEntry":
			return {
				type: "rollback_checkpoint",
				data: {
					name: kind.val.name,
				},
			};
		case "JoinEntry":
			return {
				type: "join",
				data: {
					branches: Object.fromEntries(
						Array.from(kind.val.branches.entries()).map(
							([name, status]) => [
								name,
								branchStatusFromBare(status),
							],
						),
					),
				},
			};
		case "RaceEntry":
			return {
				type: "race",
				data: {
					winner: kind.val.winner,
					branches: Object.fromEntries(
						Array.from(kind.val.branches.entries()).map(
							([name, status]) => [
								name,
								branchStatusFromBare(status),
							],
						),
					),
				},
			};
		case "RemovedEntry":
			return {
				type: "removed",
				data: {
					originalType: kind.val
						.originalType as InternalEntryKind["type"],
					originalName: kind.val.originalName ?? undefined,
				},
			};
		default:
			throw new Error(
				`Unknown entry kind: ${(kind as { tag: string }).tag}`,
			);
	}
}

// === Entry Conversion & Serialization ===

function entryToBare(entry: InternalEntry): v1.Entry {
	return {
		id: entry.id,
		location: locationToBare(entry.location),
		kind: entryKindToBare(entry.kind),
	};
}

function entryFromBare(bareEntry: v1.Entry): InternalEntry {
	return {
		id: bareEntry.id,
		location: locationFromBare(bareEntry.location),
		kind: entryKindFromBare(bareEntry.kind),
		dirty: false,
	};
}

export function serializeEntry(entry: InternalEntry): Uint8Array {
	const bareEntry = entryToBare(entry);
	return ENTRY_VERSIONED.serializeWithEmbeddedVersion(
		bareEntry,
		CURRENT_VERSION,
	);
}

export function deserializeEntry(bytes: Uint8Array): InternalEntry {
	const bareEntry = ENTRY_VERSIONED.deserializeWithEmbeddedVersion(bytes);
	return entryFromBare(bareEntry);
}

// === Entry Metadata Conversion & Serialization ===

function entryMetadataToBare(
	metadata: InternalEntryMetadata,
): v1.EntryMetadata {
	return {
		status: entryStatusToBare(metadata.status),
		error: metadata.error ?? null,
		attempts: metadata.attempts,
		lastAttemptAt: BigInt(metadata.lastAttemptAt),
		createdAt: BigInt(metadata.createdAt),
		completedAt:
			metadata.completedAt !== undefined
				? BigInt(metadata.completedAt)
				: null,
		rollbackCompletedAt:
			metadata.rollbackCompletedAt !== undefined
				? BigInt(metadata.rollbackCompletedAt)
				: null,
		rollbackError: metadata.rollbackError ?? null,
	};
}

function entryMetadataFromBare(
	bareMetadata: v1.EntryMetadata,
): InternalEntryMetadata {
	return {
		status: entryStatusFromBare(bareMetadata.status),
		error: bareMetadata.error ?? undefined,
		attempts: bareMetadata.attempts,
		lastAttemptAt: Number(bareMetadata.lastAttemptAt),
		createdAt: Number(bareMetadata.createdAt),
		completedAt:
			bareMetadata.completedAt !== null
				? Number(bareMetadata.completedAt)
				: undefined,
		rollbackCompletedAt:
			bareMetadata.rollbackCompletedAt !== null
				? Number(bareMetadata.rollbackCompletedAt)
				: undefined,
		rollbackError: bareMetadata.rollbackError ?? undefined,
		dirty: false,
	};
}

export function serializeEntryMetadata(
	metadata: InternalEntryMetadata,
): Uint8Array {
	const bareMetadata = entryMetadataToBare(metadata);
	return ENTRY_METADATA_VERSIONED.serializeWithEmbeddedVersion(
		bareMetadata,
		CURRENT_VERSION,
	);
}

export function deserializeEntryMetadata(
	bytes: Uint8Array,
): InternalEntryMetadata {
	const bareMetadata =
		ENTRY_METADATA_VERSIONED.deserializeWithEmbeddedVersion(bytes);
	return entryMetadataFromBare(bareMetadata);
}

// === Workflow Metadata Serialization ===
// Note: These are used for reading/writing individual workflow fields

export function serializeWorkflowState(
	state: InternalWorkflowState,
): Uint8Array {
	// For simple values, we can encode them directly without the full metadata struct
	// Using a single byte for efficiency
	const encoder = new TextEncoder();
	return encoder.encode(state);
}

export function deserializeWorkflowState(
	bytes: Uint8Array,
): InternalWorkflowState {
	const decoder = new TextDecoder();
	const state = decoder.decode(bytes) as InternalWorkflowState;
	const validStates: InternalWorkflowState[] = [
		"pending",
		"running",
		"sleeping",
		"failed",
		"completed",
		"cancelled",
		"rolling_back",
	];
	if (!validStates.includes(state)) {
		throw new Error(`Invalid workflow state: ${state}`);
	}
	return state;
}

export function serializeWorkflowOutput(output: unknown): Uint8Array {
	return cbor.encode(output);
}

export function deserializeWorkflowOutput<T>(bytes: Uint8Array): T {
	try {
		return cbor.decode(bytes) as T;
	} catch (error) {
		throw new Error(
			`Failed to deserialize workflow output: ${error instanceof Error ? error.message : String(error)}`,
		);
	}
}

/**
 * Structured error type for serialization.
 */
interface SerializedWorkflowError {
	name: string;
	message: string;
	stack?: string;
	metadata?: Record<string, unknown>;
}

export function serializeWorkflowError(
	error: SerializedWorkflowError,
): Uint8Array {
	return cbor.encode(error);
}

export function deserializeWorkflowError(
	bytes: Uint8Array,
): SerializedWorkflowError {
	const decoded = cbor.decode(bytes);
	assertObject(decoded, "WorkflowError");
	// Validate required fields
	const obj = decoded as Record<string, unknown>;
	assertString(obj.name, "WorkflowError.name");
	assertString(obj.message, "WorkflowError.message");
	return {
		name: obj.name,
		message: obj.message,
		stack: typeof obj.stack === "string" ? obj.stack : undefined,
		metadata:
			typeof obj.metadata === "object" && obj.metadata !== null
				? (obj.metadata as Record<string, unknown>)
				: undefined,
	};
}

export function serializeWorkflowInput(input: unknown): Uint8Array {
	return cbor.encode(input);
}

export function deserializeWorkflowInput<T>(bytes: Uint8Array): T {
	try {
		return cbor.decode(bytes) as T;
	} catch (error) {
		throw new Error(
			`Failed to deserialize workflow input: ${error instanceof Error ? error.message : String(error)}`,
		);
	}
}

// === Name Registry Serialization ===

export function serializeName(name: string): Uint8Array {
	const encoder = new TextEncoder();
	return encoder.encode(name);
}

export function deserializeName(bytes: Uint8Array): string {
	const decoder = new TextDecoder();
	return decoder.decode(bytes);
}
