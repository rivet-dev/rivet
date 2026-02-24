import type { Edge, Node } from "@xyflow/react";
import type {
	EntryKindType,
	EntryStatus,
	ExtendedEntryType,
	HistoryItem,
	JoinEntry,
	LoopEntry,
	LoopIterationMarker,
	MessageEntry,
	RaceEntry,
	RemovedEntry,
	SleepEntry,
	StepEntry,
	WorkflowHistory,
} from "./workflow-types";
import {
	type BranchGroupNodeData,
	formatDuration,
	LOOP_HEADER_HEIGHT,
	LOOP_PADDING_BOTTOM,
	LOOP_PADDING_X,
	type LoopGroupNodeData,
	NODE_HEIGHT,
	NODE_WIDTH,
	TERMINATION_NODE_SIZE,
	type TerminationNodeData,
	type WorkflowNodeData,
} from "./xyflow-nodes";

// ─── Constants ───────────────────────────────────────────────

const NODE_GAP_Y = 48;
const BRANCH_GAP_X = 60;
const GROUP_WIDTH = NODE_WIDTH + 2 * LOOP_PADDING_X;

// ─── Node types ──────────────────────────────────────────────

type XYNode = Node<WorkflowNodeData, "workflow">;
type XYLoopGroupNode = Node<LoopGroupNodeData, "loopGroup">;
type XYBranchGroupNode = Node<BranchGroupNodeData, "branchGroup">;
type XYTerminationNode = Node<TerminationNodeData, "termination">;
type AnyXYNode =
	| XYNode
	| XYLoopGroupNode
	| XYBranchGroupNode
	| XYTerminationNode;

export interface LayoutResult {
	nodes: AnyXYNode[];
	edges: Edge[];
}

// ─── Helpers ─────────────────────────────────────────────────

function getDisplayName(key: string): string {
	const parts = key.split("/");
	return parts[parts.length - 1].replace(/^~\d+\//, "");
}

function getEntrySummary(type: ExtendedEntryType, data: unknown): string {
	switch (type) {
		case "step": {
			const d = data as StepEntry;
			if (d.error) return "error";
			if (d.output === true) return "success";
			if (typeof d.output === "number") return String(d.output);
			return "completed";
		}
		case "sleep":
			return (data as SleepEntry).state;
		case "message":
			return (data as MessageEntry).name.split(":").pop() || "received";
		case "loop":
			return `${(data as LoopEntry).iteration} iterations`;
		case "rollback_checkpoint":
			return "checkpoint";
		case "join": {
			const d = data as JoinEntry;
			const done = Object.values(d.branches).filter(
				(b) => b.status === "completed",
			).length;
			return `${done}/${Object.keys(d.branches).length} done`;
		}
		case "race": {
			const d = data as RaceEntry;
			return d.winner ? `winner: ${d.winner}` : "racing";
		}
		case "removed":
			return (data as RemovedEntry).originalType;
		case "input":
		case "output": {
			const d = data as { value: unknown };
			if (typeof d.value === "object" && d.value !== null) {
				const keys = Object.keys(d.value);
				return keys.length > 0 ? `${keys.length} fields` : "empty";
			}
			return String(d.value);
		}
		default:
			return "";
	}
}

/** Extract common node properties from a HistoryItem. */
function itemToNodeData(item: HistoryItem) {
	const {
		startedAt,
		completedAt,
		kind,
		status: rawStatus,
		retryCount,
		error,
	} = item.entry;
	const duration =
		startedAt && completedAt ? completedAt - startedAt : undefined;
	const status: EntryStatus =
		rawStatus ||
		(completedAt ? "completed" : startedAt ? "running" : "completed");
	return {
		name: getDisplayName(item.key),
		summary: getEntrySummary(kind.type, kind.data),
		entryType: kind.type,
		status,
		duration,
		startedAt,
		completedAt,
		retryCount,
		error,
		rawData: kind.data,
		nodeKey: item.key,
	};
}

/** Sort history items by their last location segment. */
function sortByLocation(items: HistoryItem[]) {
	items.sort((a, b) => {
		const aLoc = a.entry.location[a.entry.location.length - 1] as number;
		const bLoc = b.entry.location[b.entry.location.length - 1] as number;
		return aLoc - bLoc;
	});
}

/** Calculate the height of a group container given a child count. */
function groupHeight(childCount: number): number {
	if (childCount <= 0) return LOOP_HEADER_HEIGHT + LOOP_PADDING_BOTTOM;
	return (
		LOOP_HEADER_HEIGHT +
		childCount * NODE_HEIGHT +
		(childCount - 1) * NODE_GAP_Y +
		LOOP_PADDING_BOTTOM
	);
}

/** Create a workflow node at the given position. */
function makeNode(
	id: string,
	x: number,
	y: number,
	data: Omit<
		WorkflowNodeData,
		"label" | "summary" | "entryType" | "status"
	> & {
		label?: string;
		summary?: string;
		entryType: EntryKindType | "input" | "output";
		status: EntryStatus;
		name?: string;
	},
): XYNode {
	return {
		id,
		type: "workflow",
		position: { x, y },
		measured: { width: NODE_WIDTH, height: NODE_HEIGHT },
		data: {
			label: data.label ?? data.name ?? "",
			summary: data.summary ?? "",
			entryType: data.entryType,
			status: data.status,
			duration: data.duration,
			retryCount: data.retryCount,
			error: data.error,
			nodeKey: data.nodeKey,
			startedAt: data.startedAt,
			completedAt: data.completedAt,
			rawData: data.rawData,
		},
	};
}

/** Create a child node inside a parent group. */
function makeChildNode(
	id: string,
	parentId: string,
	y: number,
	data: Parameters<typeof makeNode>[3],
): XYNode {
	const node = makeNode(id, LOOP_PADDING_X, y, data);
	(node as XYNode & { parentId: string }).parentId = parentId;
	(node as XYNode & { extent: string }).extent = "parent";
	return node;
}

// ─── Main transform ──────────────────────────────────────────

export function workflowHistoryToXYFlow(
	history: WorkflowHistory,
): LayoutResult {
	const nodes: AnyXYNode[] = [];
	const edges: Edge[] = [];

	// Partition items into top-level and nested (grouped by parent key).
	const topLevel: HistoryItem[] = [];
	const nestedByParent = new Map<string, HistoryItem[]>();

	for (const item of history.history) {
		const loc = item.entry.location;
		if (loc.length === 1 && typeof loc[0] === "number") {
			topLevel.push(item);
		} else {
			const parentKey = item.key.split("/")[0];
			const siblings = nestedByParent.get(parentKey) ?? [];
			siblings.push(item);
			nestedByParent.set(parentKey, siblings);
		}
	}

	sortByLocation(topLevel);

	// Cursor state for sequential layout.
	let currentY = 0;
	let prevNodeId: string | null = null;
	let prevCompletedAt: number | undefined;
	let pendingBranchSources: string[] = [];

	/** Connect one or more predecessors to a target node, with optional gap labels. */
	function connectTo(targetId: string, targetStartedAt?: number) {
		if (prevNodeId) {
			const gap =
				prevCompletedAt &&
					targetStartedAt &&
					targetStartedAt > prevCompletedAt
					? formatDuration(targetStartedAt - prevCompletedAt)
					: undefined;
			edges.push({
				id: `e-${prevNodeId}-${targetId}`,
				source: prevNodeId,
				target: targetId,
				...(gap && {
					label: gap,
					style: { stroke: "hsl(var(--muted-foreground))" },
					labelStyle: {
						fill: "hsl(var(--muted-foreground))",
						fontSize: 10,
					},
					labelBgStyle: {
						fill: "hsl(var(--background))",
						fillOpacity: 0.8,
					},
				}),
			});
		}
		for (const srcId of pendingBranchSources) {
			edges.push({
				id: `e-${srcId}-${targetId}`,
				source: srcId,
				target: targetId,
			});
		}
		pendingBranchSources = [];
	}

	/** Place a sequential node, connect it, and advance the cursor. */
	function addSequentialNode(
		id: string,
		data: Parameters<typeof makeNode>[3],
		startedAt?: number,
	) {
		nodes.push(makeNode(id, 0, currentY, data));
		connectTo(id, startedAt);
		prevNodeId = id;
		prevCompletedAt = data.completedAt;
		currentY += NODE_HEIGHT + NODE_GAP_Y;
	}

	/** Chain a list of child nodes inside a parent group, connecting them sequentially. Returns the last child id. */
	function addChildChain(
		items: HistoryItem[],
		parentId: string,
		startY: number,
	): { lastChildId: string | null; endY: number } {
		let y = startY;
		let lastId: string | null = null;

		for (const item of items) {
			const d = itemToNodeData(item);
			const id = `child-${item.entry.id}`;
			nodes.push(makeChildNode(id, parentId, y, { ...d, label: d.name }));

			if (lastId) {
				edges.push({
					id: `e-${lastId}-${id}`,
					source: lastId,
					target: id,
				});
			}
			lastId = id;
			y += NODE_HEIGHT + NODE_GAP_Y;
		}

		return { lastChildId: lastId, endY: y };
	}

	// ── Input meta node ──

	if (history.input !== undefined) {
		addSequentialNode("meta-input", {
			label: "Input",
			summary: getEntrySummary("input", { value: history.input }),
			entryType: "input",
			status: "completed",
			nodeKey: "input",
			rawData: { value: history.input },
		});
	}

	// ── Main loop over top-level items ──

	for (const item of topLevel) {
		const entryType = item.entry.kind.type;
		const d = itemToNodeData(item);

		if (entryType === "loop") {
			const loopId = `loop-${item.entry.id}`;
			const children = collectLoopChildren(
				nestedByParent.get(item.key) ?? [],
			);
			const height = groupHeight(children.length);

			nodes.push({
				id: loopId,
				type: "loopGroup",
				position: { x: -LOOP_PADDING_X, y: currentY },
				measured: { width: GROUP_WIDTH, height },
				style: { width: GROUP_WIDTH, height },
				data: { label: d.name, summary: d.summary },
			} as XYLoopGroupNode);

			connectTo(loopId, d.startedAt);

			const { lastChildId } = addChildChain(
				children,
				loopId,
				LOOP_HEADER_HEIGHT,
			);

			currentY += height + NODE_GAP_Y;
			prevNodeId = lastChildId ?? loopId;
			prevCompletedAt = d.completedAt;
		} else if (entryType === "join" || entryType === "race") {
			// Header node for the join/race.
			const headerId = `header-${item.entry.id}`;
			addSequentialNode(headerId, { ...d, label: d.name });

			const branchData = item.entry.kind.data as JoinEntry | RaceEntry;
			const branchNames = Object.keys(branchData.branches);
			const nested = nestedByParent.get(item.key) ?? [];

			// Build per-branch info.
			const TERMINATION_GAP = 24;

			const branches = branchNames.map((name) => {
				const branchItems = nested.filter((ni) =>
					ni.key.includes(`/${name}/`),
				);
				sortByLocation(branchItems);
				const status = branchData.branches[name].status;
				const isFailed = status === "failed" || status === "cancelled";
				return {
					name,
					items: branchItems,
					status,
					isFailed,
					height: groupHeight(branchItems.length),
				};
			});

			const maxHeight = Math.max(...branches.map((b) => b.height));
			const totalWidth =
				branches.length * GROUP_WIDTH +
				(branches.length - 1) * BRANCH_GAP_X;
			const startX = -totalWidth / 2 + GROUP_WIDTH / 2 - LOOP_PADDING_X;
			const branchStartY = currentY;
			const branchGroupIds: string[] = [];

			for (let i = 0; i < branches.length; i++) {
				const branch = branches[i];
				const branchX = startX + i * (GROUP_WIDTH + BRANCH_GAP_X);
				const groupId = `branchgroup-${item.entry.id}-${branch.name}`;

				nodes.push({
					id: groupId,
					type: "branchGroup",
					position: { x: branchX, y: branchStartY },
					measured: { width: GROUP_WIDTH, height: maxHeight },
					style: { width: GROUP_WIDTH, height: maxHeight },
					data: {
						label: branch.name,
						entryType: entryType as "join" | "race",
						branchStatus: branch.status,
					},
				} as XYBranchGroupNode);

				edges.push({
					id: `e-${headerId}-${groupId}`,
					source: headerId,
					target: groupId,
				});

				addChildChain(branch.items, groupId, LOOP_HEADER_HEIGHT);

				if (!branch.isFailed) {
					branchGroupIds.push(groupId);
				}
			}

			// Place termination nodes below failed/cancelled branch groups.
			const hasTerminations = branches.some((b) => b.isFailed);
			const termY = branchStartY + maxHeight + TERMINATION_GAP;

			for (let i = 0; i < branches.length; i++) {
				const branch = branches[i];
				if (!branch.isFailed) continue;

				const branchX = startX + i * (GROUP_WIDTH + BRANCH_GAP_X);
				const groupId = `branchgroup-${item.entry.id}-${branch.name}`;
				const termId = `term-${item.entry.id}-${branch.name}`;
				const termX =
					branchX + GROUP_WIDTH / 2 - TERMINATION_NODE_SIZE / 2;

				nodes.push({
					id: termId,
					type: "termination",
					position: { x: termX, y: termY },
					measured: {
						width: TERMINATION_NODE_SIZE,
						height: TERMINATION_NODE_SIZE,
					},
					data: {},
				} as XYTerminationNode);

				edges.push({
					id: `e-${groupId}-${termId}`,
					source: groupId,
					target: termId,
				});
			}

			currentY = hasTerminations
				? termY + TERMINATION_NODE_SIZE + NODE_GAP_Y
				: branchStartY + maxHeight + NODE_GAP_Y;
			prevNodeId = null;
			prevCompletedAt = d.completedAt;
			pendingBranchSources = branchGroupIds;
		} else {
			addSequentialNode(`node-${item.entry.id}`, { ...d, label: d.name });
		}
	}

	// ── Output meta node ──

	if (history.output !== undefined && history.state === "completed") {
		const id = "meta-output";
		nodes.push(
			makeNode(id, 0, currentY, {
				label: "Output",
				summary: getEntrySummary("output", { value: history.output }),
				entryType: "output",
				status: "completed",
				nodeKey: "output",
				rawData: { value: history.output },
			}),
		);
		connectTo(id);
	}

	return { nodes, edges };
}

// ─── Loop child collection ───────────────────────────────────

/** Collect loop children from nested items, flattened across iterations and sorted. */
function collectLoopChildren(items: HistoryItem[]): HistoryItem[] {
	const iterationMap = new Map<number, HistoryItem[]>();
	for (const item of items) {
		const marker = item.entry.location.find(
			(s): s is LoopIterationMarker =>
				typeof s === "object" && "iteration" in s,
		);
		if (marker) {
			const list = iterationMap.get(marker.iteration) ?? [];
			list.push(item);
			iterationMap.set(marker.iteration, list);
		}
	}

	const result: HistoryItem[] = [];
	const sortedKeys = Array.from(iterationMap.keys()).sort((a, b) => a - b);
	for (const key of sortedKeys) {
		const iterItems = iterationMap.get(key) ?? [];
		sortByLocation(iterItems);
		result.push(...iterItems);
	}
	return result;
}
