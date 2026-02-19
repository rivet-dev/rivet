import * as cbor from "cbor-x";
import type {
	TransportWorkflowHistory,
} from "rivetkit/inspector/client";
import type {
	BranchStatus,
	BranchStatusType,
	EntryKind,
	EntryKindType,
	EntryStatus,
	Location,
	SleepState,
	WorkflowHistory,
} from "./workflow-types";

type TransportWorkflowEntry = TransportWorkflowHistory["entries"][number];

function decodeCborOrNull(data: ArrayBuffer | null): unknown {
	if (data === null) return undefined;
	try {
		return cbor.decode(new Uint8Array(data));
	} catch {
		return undefined;
	}
}

function transformLocation(
	location: TransportWorkflowEntry["location"],
): Location {
	return location.map((segment) => {
		if (segment.tag === "WorkflowNameIndex") {
			return segment.val;
		}
		return {
			loop: segment.val.loop,
			iteration: segment.val.iteration,
		};
	});
}

function transformSleepState(state: string): SleepState {
	switch (state) {
		case "PENDING":
			return "pending";
		case "COMPLETED":
			return "completed";
		case "INTERRUPTED":
			return "interrupted";
		default:
			return "pending";
	}
}

function transformBranchStatusType(status: string): BranchStatusType {
	switch (status) {
		case "PENDING":
			return "pending";
		case "RUNNING":
			return "running";
		case "COMPLETED":
			return "completed";
		case "FAILED":
			return "failed";
		case "CANCELLED":
			return "cancelled";
		default:
			return "pending";
	}
}

function transformBranches(
	branches: ReadonlyMap<
		string,
		{ status: string; output: ArrayBuffer | null; error: string | null }
	>,
): Record<string, BranchStatus> {
	const result: Record<string, BranchStatus> = {};
	for (const [name, branch] of branches) {
		result[name] = {
			status: transformBranchStatusType(branch.status),
			output: decodeCborOrNull(branch.output),
			error: branch.error ?? undefined,
		};
	}
	return result;
}

function transformEntryKind(kind: TransportWorkflowEntry["kind"]): EntryKind {
	switch (kind.tag) {
		case "WorkflowStepEntry":
			return {
				type: "step",
				data: {
					output: decodeCborOrNull(kind.val.output),
					error: kind.val.error ?? undefined,
				},
			};
		case "WorkflowLoopEntry":
			return {
				type: "loop",
				data: {
					state: decodeCborOrNull(kind.val.state),
					iteration: kind.val.iteration,
					output: decodeCborOrNull(kind.val.output),
				},
			};
		case "WorkflowSleepEntry":
			return {
				type: "sleep",
				data: {
					deadline: Number(kind.val.deadline),
					state: transformSleepState(kind.val.state),
				},
			};
		case "WorkflowMessageEntry":
			return {
				type: "message",
				data: {
					name: kind.val.name,
					data: decodeCborOrNull(kind.val.messageData),
				},
			};
		case "WorkflowRollbackCheckpointEntry":
			return {
				type: "rollback_checkpoint",
				data: { name: kind.val.name },
			};
		case "WorkflowJoinEntry":
			return {
				type: "join",
				data: { branches: transformBranches(kind.val.branches) },
			};
		case "WorkflowRaceEntry":
			return {
				type: "race",
				data: {
					winner: kind.val.winner,
					branches: transformBranches(kind.val.branches),
				},
			};
		case "WorkflowRemovedEntry":
			return {
				type: "removed",
				data: {
					originalType: kind.val.originalType as EntryKindType,
					originalName: kind.val.originalName ?? undefined,
				},
			};
	}
}

function transformEntryStatus(status: string): EntryStatus {
	switch (status) {
		case "PENDING":
			return "pending";
		case "RUNNING":
			return "running";
		case "COMPLETED":
			return "completed";
		case "FAILED":
			return "failed";
		case "EXHAUSTED":
			return "retrying";
		default:
			return "pending";
	}
}

function buildEntryKey(
	location: Location,
	nameRegistry: readonly string[],
): string {
	return location
		.map((segment) => {
			if (typeof segment === "number") {
				return nameRegistry[segment] ?? `unknown-${segment}`;
			}
			const loopName =
				nameRegistry[segment.loop] ?? `loop-${segment.loop}`;
			return `${loopName}[${segment.iteration}]`;
		})
		.join("/");
}

/**
 * Transform a decoded TransportWorkflowHistory into the UI WorkflowHistory format.
 */
export function transformWorkflowHistory(
	transport: TransportWorkflowHistory,
): WorkflowHistory {
	const { nameRegistry, entries, entryMetadata } = transport;

	const history = entries.map((entry) => {
		const location = transformLocation(entry.location);
		const meta = entryMetadata.get(entry.id);
		const key = buildEntryKey(location, nameRegistry);

		return {
			key,
			entry: {
				id: entry.id,
				location,
				kind: transformEntryKind(entry.kind),
				dirty: false,
				status: meta
					? transformEntryStatus(meta.status)
					: ("pending" as EntryStatus),
				startedAt: meta ? Number(meta.createdAt) : undefined,
				completedAt:
					meta?.completedAt != null
						? Number(meta.completedAt)
						: undefined,
				retryCount: meta ? meta.attempts : undefined,
				error: meta?.error ?? undefined,
			},
		};
	});

	// Derive the overall workflow state from entry metadata.
	const hasRunning = history.some((h) => h.entry.status === "running");
	const hasFailed = history.some((h) => h.entry.status === "failed");
	const hasPending = history.some((h) => h.entry.status === "pending");
	const allCompleted =
		history.length > 0 &&
		history.every((h) => h.entry.status === "completed");

	let state: WorkflowHistory["state"] = "pending";
	if (allCompleted) {
		state = "completed";
	} else if (hasFailed) {
		state = "failed";
	} else if (hasRunning) {
		state = "running";
	} else if (
		hasPending &&
		history.some((h) => h.entry.status === "completed")
	) {
		state = "running";
	}

	return {
		workflowId: entries[0]?.id ?? "unknown",
		state,
		nameRegistry: [...nameRegistry],
		history,
	};
}
