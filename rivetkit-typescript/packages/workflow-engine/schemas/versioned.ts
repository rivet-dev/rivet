import { createVersionedDataHandler } from "vbare";
import * as v1 from "../dist/schemas/v1.js";

export const CURRENT_VERSION = 1;

// Re-export generated types for convenience
export type {
	Entry,
	EntryMetadata,
	Signal,
	WorkflowMetadata,
	EntryKind,
	StepEntry,
	LoopEntry,
	SleepEntry,
	SignalEntry,
	JoinEntry,
	RaceEntry,
	RemovedEntry,
	BranchStatus,
	Location,
	PathSegment,
	LoopIterationMarker,
} from "../dist/schemas/v1.js";

export {
	EntryStatus,
	SleepState,
	BranchStatusType,
	WorkflowState,
} from "../dist/schemas/v1.js";

// === Entry Handler ===

export const ENTRY_VERSIONED = createVersionedDataHandler<v1.Entry>({
	deserializeVersion: (bytes, version) => {
		switch (version) {
			case 1:
				return v1.decodeEntry(bytes);
			default:
				throw new Error(`Unknown Entry version ${version}`);
		}
	},
	serializeVersion: (data, version) => {
		switch (version) {
			case 1:
				return v1.encodeEntry(data as v1.Entry);
			default:
				throw new Error(`Unknown Entry version ${version}`);
		}
	},
	deserializeConverters: () => [],
	serializeConverters: () => [],
});

// === Entry Metadata Handler ===

export const ENTRY_METADATA_VERSIONED = createVersionedDataHandler<v1.EntryMetadata>({
	deserializeVersion: (bytes, version) => {
		switch (version) {
			case 1:
				return v1.decodeEntryMetadata(bytes);
			default:
				throw new Error(`Unknown EntryMetadata version ${version}`);
		}
	},
	serializeVersion: (data, version) => {
		switch (version) {
			case 1:
				return v1.encodeEntryMetadata(data as v1.EntryMetadata);
			default:
				throw new Error(`Unknown EntryMetadata version ${version}`);
		}
	},
	deserializeConverters: () => [],
	serializeConverters: () => [],
});

// === Signal Handler ===

export const SIGNAL_VERSIONED = createVersionedDataHandler<v1.Signal>({
	deserializeVersion: (bytes, version) => {
		switch (version) {
			case 1:
				return v1.decodeSignal(bytes);
			default:
				throw new Error(`Unknown Signal version ${version}`);
		}
	},
	serializeVersion: (data, version) => {
		switch (version) {
			case 1:
				return v1.encodeSignal(data as v1.Signal);
			default:
				throw new Error(`Unknown Signal version ${version}`);
		}
	},
	deserializeConverters: () => [],
	serializeConverters: () => [],
});

// === Workflow Metadata Handler ===

export const WORKFLOW_METADATA_VERSIONED = createVersionedDataHandler<v1.WorkflowMetadata>({
	deserializeVersion: (bytes, version) => {
		switch (version) {
			case 1:
				return v1.decodeWorkflowMetadata(bytes);
			default:
				throw new Error(`Unknown WorkflowMetadata version ${version}`);
		}
	},
	serializeVersion: (data, version) => {
		switch (version) {
			case 1:
				return v1.encodeWorkflowMetadata(data as v1.WorkflowMetadata);
			default:
				throw new Error(`Unknown WorkflowMetadata version ${version}`);
		}
	},
	deserializeConverters: () => [],
	serializeConverters: () => [],
});
