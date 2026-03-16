import * as cbor from "cbor-x";
import { decodeWorkflowHistoryTransport } from "@/inspector/transport";
import type * as transport from "@/schemas/transport/mod";

function decodeWorkflowCbor(data: ArrayBuffer | null): unknown | null {
	if (data === null) {
		return null;
	}

	try {
		return cbor.decode(new Uint8Array(data));
	} catch {
		return null;
	}
}

function serializeWorkflowLocation(
	location: transport.WorkflowLocation,
): Array<
	| { tag: "WorkflowNameIndex"; val: number }
	| {
			tag: "WorkflowLoopIterationMarker";
			val: { loop: number; iteration: number };
	  }
> {
	return location.map((segment) => {
		if (segment.tag === "WorkflowNameIndex") {
			return {
				tag: segment.tag,
				val: segment.val,
			};
		}

		return {
			tag: segment.tag,
			val: {
				loop: segment.val.loop,
				iteration: segment.val.iteration,
			},
		};
	});
}

function serializeWorkflowBranches(
	branches: ReadonlyMap<string, transport.WorkflowBranchStatus>,
): Record<
	string,
	{ status: string; output: unknown | null; error: string | null }
> {
	return Object.fromEntries(
		Array.from(branches.entries()).map(([name, branch]) => [
			name,
			{
				status: branch.status,
				output: decodeWorkflowCbor(branch.output),
				error: branch.error,
			},
		]),
	);
}

function serializeWorkflowEntryKind(kind: transport.WorkflowEntryKind):
	| {
			tag: "WorkflowStepEntry";
			val: { output: unknown | null; error: string | null };
	  }
	| {
			tag: "WorkflowLoopEntry";
			val: {
				state: unknown | null;
				iteration: number;
				output: unknown | null;
			};
	  }
	| {
			tag: "WorkflowSleepEntry";
			val: { deadline: number; state: string };
	  }
	| {
			tag: "WorkflowMessageEntry";
			val: { name: string; messageData: unknown | null };
	  }
	| {
			tag: "WorkflowRollbackCheckpointEntry";
			val: { name: string };
	  }
	| {
			tag: "WorkflowJoinEntry";
			val: {
				branches: Record<
					string,
					{
						status: string;
						output: unknown | null;
						error: string | null;
					}
				>;
			};
	  }
	| {
			tag: "WorkflowRaceEntry";
			val: {
				winner: string | null;
				branches: Record<
					string,
					{
						status: string;
						output: unknown | null;
						error: string | null;
					}
				>;
			};
	  }
	| {
			tag: "WorkflowRemovedEntry";
			val: { originalType: string; originalName: string | null };
	  } {
	switch (kind.tag) {
		case "WorkflowStepEntry":
			return {
				tag: kind.tag,
				val: {
					output: decodeWorkflowCbor(kind.val.output),
					error: kind.val.error,
				},
			};
		case "WorkflowLoopEntry":
			return {
				tag: kind.tag,
				val: {
					state: decodeWorkflowCbor(kind.val.state),
					iteration: kind.val.iteration,
					output: decodeWorkflowCbor(kind.val.output),
				},
			};
		case "WorkflowSleepEntry":
			return {
				tag: kind.tag,
				val: {
					deadline: Number(kind.val.deadline),
					state: kind.val.state,
				},
			};
		case "WorkflowMessageEntry":
			return {
				tag: kind.tag,
				val: {
					name: kind.val.name,
					messageData: decodeWorkflowCbor(kind.val.messageData),
				},
			};
		case "WorkflowRollbackCheckpointEntry":
			return {
				tag: kind.tag,
				val: {
					name: kind.val.name,
				},
			};
		case "WorkflowJoinEntry":
			return {
				tag: kind.tag,
				val: {
					branches: serializeWorkflowBranches(kind.val.branches),
				},
			};
		case "WorkflowRaceEntry":
			return {
				tag: kind.tag,
				val: {
					winner: kind.val.winner,
					branches: serializeWorkflowBranches(kind.val.branches),
				},
			};
		case "WorkflowRemovedEntry":
			return {
				tag: kind.tag,
				val: {
					originalType: kind.val.originalType,
					originalName: kind.val.originalName,
				},
			};
	}
}

export function serializeWorkflowHistoryForJson(
	data: ArrayBuffer | null,
):
	| {
			nameRegistry: string[];
			entries: Array<{
				id: string;
				location: Array<
					| { tag: "WorkflowNameIndex"; val: number }
					| {
							tag: "WorkflowLoopIterationMarker";
							val: { loop: number; iteration: number };
					  }
				>;
				kind:
					| {
							tag: "WorkflowStepEntry";
							val: { output: unknown | null; error: string | null };
					  }
					| {
							tag: "WorkflowLoopEntry";
							val: {
								state: unknown | null;
								iteration: number;
								output: unknown | null;
							};
					  }
					| {
							tag: "WorkflowSleepEntry";
							val: { deadline: number; state: string };
					  }
					| {
							tag: "WorkflowMessageEntry";
							val: { name: string; messageData: unknown | null };
					  }
					| {
							tag: "WorkflowRollbackCheckpointEntry";
							val: { name: string };
					  }
					| {
							tag: "WorkflowJoinEntry";
							val: {
								branches: Record<
									string,
									{
										status: string;
										output: unknown | null;
										error: string | null;
									}
								>;
							};
					  }
					| {
							tag: "WorkflowRaceEntry";
							val: {
								winner: string | null;
								branches: Record<
									string,
									{
										status: string;
										output: unknown | null;
										error: string | null;
									}
								>;
							};
					  }
					| {
							tag: "WorkflowRemovedEntry";
							val: {
								originalType: string;
								originalName: string | null;
							};
					  };
			}>;
			entryMetadata: Record<
				string,
				{
					status: string;
					error: string | null;
					attempts: number;
					lastAttemptAt: number;
					createdAt: number;
					completedAt: number | null;
					rollbackCompletedAt: number | null;
					rollbackError: string | null;
				}
			>;
	  }
	| null {
	if (data === null) {
		return null;
	}

	const history = decodeWorkflowHistoryTransport(data);

	return {
		nameRegistry: [...history.nameRegistry],
		entries: history.entries.map((entry) => ({
			id: entry.id,
			location: serializeWorkflowLocation(entry.location),
			kind: serializeWorkflowEntryKind(entry.kind),
		})),
		entryMetadata: Object.fromEntries(
			Array.from(history.entryMetadata.entries()).map(([entryId, meta]) => [
				entryId,
				{
					status: meta.status,
					error: meta.error,
					attempts: meta.attempts,
					lastAttemptAt: Number(meta.lastAttemptAt),
					createdAt: Number(meta.createdAt),
					completedAt:
						meta.completedAt === null
							? null
							: Number(meta.completedAt),
					rollbackCompletedAt:
						meta.rollbackCompletedAt === null
							? null
							: Number(meta.rollbackCompletedAt),
					rollbackError: meta.rollbackError,
				},
			]),
		),
	};
}
