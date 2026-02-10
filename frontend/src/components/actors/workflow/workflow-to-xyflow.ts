import type { Edge, Node } from "@xyflow/react";
import {
	NODE_HEIGHT,
	NODE_WIDTH,
	LOOP_HEADER_HEIGHT,
	LOOP_PADDING_X,
	LOOP_PADDING_BOTTOM,
	type WorkflowNodeData,
	type LoopGroupNodeData,
	type BranchGroupNodeData,
	formatDuration,
} from "./xyflow-nodes";
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

const NODE_GAP_Y = 48;
const BRANCH_GAP_X = 60;

function getDisplayName(key: string): string {
	const parts = key.split("/");
	let name = parts[parts.length - 1];
	name = name.replace(/^~\d+\//, "");
	return name;
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
		case "sleep": {
			const d = data as SleepEntry;
			return d.state;
		}
		case "message": {
			const d = data as MessageEntry;
			return d.name.split(":").pop() || "received";
		}
		case "loop": {
			const d = data as LoopEntry;
			return `${d.iteration} iterations`;
		}
		case "rollback_checkpoint":
			return "checkpoint";
		case "join": {
			const d = data as JoinEntry;
			const completed = Object.values(d.branches).filter(
				(b) => b.status === "completed",
			).length;
			return `${completed}/${Object.keys(d.branches).length} done`;
		}
		case "race": {
			const d = data as RaceEntry;
			return d.winner ? `winner: ${d.winner}` : "racing";
		}
		case "removed": {
			const d = data as RemovedEntry;
			return d.originalType;
		}
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

type XYNode = Node<WorkflowNodeData, "workflow">;
type XYLoopGroupNode = Node<LoopGroupNodeData, "loopGroup">;
type XYBranchGroupNode = Node<BranchGroupNodeData, "branchGroup">;
type AnyXYNode = XYNode | XYLoopGroupNode | XYBranchGroupNode;

interface LayoutResult {
	nodes: AnyXYNode[];
	edges: Edge[];
}

function makeNode(
	id: string,
	x: number,
	y: number,
	label: string,
	summary: string,
	entryType: EntryKindType | "input" | "output",
	status: EntryStatus,
	opts?: {
		duration?: number;
		retryCount?: number;
		error?: string;
	},
): XYNode {
	return {
		id,
		type: "workflow",
		position: { x, y },
		measured: { width: NODE_WIDTH, height: NODE_HEIGHT },
		data: {
			label,
			summary,
			entryType,
			status,
			duration: opts?.duration,
			retryCount: opts?.retryCount,
			error: opts?.error,
		},
	};
}

export function workflowHistoryToXYFlow(
	history: WorkflowHistory,
): LayoutResult {
	const { history: items } = history;
	const nodes: AnyXYNode[] = [];
	const edges: Edge[] = [];

	// Separate top-level from nested items.
	const topLevel: HistoryItem[] = [];
	const nestedByParent = new Map<string, HistoryItem[]>();

	for (const item of items) {
		const loc = item.entry.location;
		if (loc.length === 1 && typeof loc[0] === "number") {
			topLevel.push(item);
		} else {
			const parentKey = item.key.split("/")[0];
			if (!nestedByParent.has(parentKey)) {
				nestedByParent.set(parentKey, []);
			}
			nestedByParent.get(parentKey)?.push(item);
		}
	}

	topLevel.sort(
		(a, b) =>
			(a.entry.location[0] as number) - (b.entry.location[0] as number),
	);

	const centerX = 0;
	let currentY = 0;
	let prevNodeId: string | null = null;
	let prevCompletedAt: number | undefined;
	let pendingBranchSources: string[] = [];

	function connectToTarget(targetId: string, targetStartedAt?: number) {
		const gapLabel =
			prevCompletedAt && targetStartedAt && targetStartedAt > prevCompletedAt
				? formatDuration(targetStartedAt - prevCompletedAt)
				: undefined;

		if (prevNodeId) {
			edges.push({
				id: `e-${prevNodeId}-${targetId}`,
				source: prevNodeId,
				target: targetId,
				...(gapLabel && {
					label: gapLabel,
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

	// Input meta node.
	if (history.input !== undefined) {
		const id = "meta-input";
		nodes.push(
			makeNode(
				id,
				centerX,
				currentY,
				"Input",
				getEntrySummary("input", { value: history.input }),
				"input",
				"completed",
			),
		);
		prevNodeId = id;
		currentY += NODE_HEIGHT + NODE_GAP_Y;
	}

	for (const item of topLevel) {
		const entryType = item.entry.kind.type;
		const startedAt = item.entry.startedAt;
		const completedAt = item.entry.completedAt;
		const duration =
			startedAt && completedAt ? completedAt - startedAt : undefined;
		const status: EntryStatus =
			item.entry.status ||
			(completedAt ? "completed" : startedAt ? "running" : "pending");
		const name = getDisplayName(item.key);
		const summary = getEntrySummary(entryType, item.entry.kind.data);

		if (entryType === "loop") {
			// Loop: render a group container with iteration children inside.
			const loopNodeId = `loop-${item.entry.id}`;

			// Gather and flatten all iteration items to count children.
			const loopItems = nestedByParent.get(item.key) || [];
			const iterationMap = new Map<number, HistoryItem[]>();
			for (const li of loopItems) {
				const marker = li.entry.location.find(
					(s): s is LoopIterationMarker =>
						typeof s === "object" && "iteration" in s,
				);
				if (marker) {
					if (!iterationMap.has(marker.iteration)) {
						iterationMap.set(marker.iteration, []);
					}
					iterationMap.get(marker.iteration)?.push(li);
				}
			}

			const sortedIterations = Array.from(iterationMap.entries()).sort(
				(a, b) => a[0] - b[0],
			);

			// Count total children across all iterations.
			let childCount = 0;
			for (const [, iterItems] of sortedIterations) {
				childCount += iterItems.length;
			}

			// Calculate group container dimensions.
			const loopWidth = NODE_WIDTH + 2 * LOOP_PADDING_X;
			const loopHeight =
				childCount > 0
					? LOOP_HEADER_HEIGHT +
						childCount * NODE_HEIGHT +
						(childCount - 1) * NODE_GAP_Y +
						LOOP_PADDING_BOTTOM
					: LOOP_HEADER_HEIGHT + LOOP_PADDING_BOTTOM;

			// The group node must come before its children in the array.
			// Center the group so children align with the main flow.
			const loopX = centerX - LOOP_PADDING_X;
			nodes.push({
				id: loopNodeId,
				type: "loopGroup",
				position: { x: loopX, y: currentY },
				measured: { width: loopWidth, height: loopHeight },
				style: { width: loopWidth, height: loopHeight },
				data: {
					label: name,
					summary: summary,
				},
			} as XYLoopGroupNode);

			connectToTarget(loopNodeId, startedAt);

			// Place children relative to the group container.
			let childRelY = LOOP_HEADER_HEIGHT;
			let loopLastChildId: string | null = null;

			for (const [, iterItems] of sortedIterations) {
				iterItems.sort((a, b) => {
					const aLoc = a.entry.location[
						a.entry.location.length - 1
					] as number;
					const bLoc = b.entry.location[
						b.entry.location.length - 1
					] as number;
					return aLoc - bLoc;
				});

				for (const li of iterItems) {
					const liStartedAt = li.entry.startedAt;
					const liCompletedAt = li.entry.completedAt;
					const liDuration =
						liStartedAt && liCompletedAt
							? liCompletedAt - liStartedAt
							: undefined;
					const liStatus: EntryStatus =
						li.entry.status || "completed";
					const liId = `iter-${li.entry.id}`;
					const liName = getDisplayName(li.key);
					const liSummary = getEntrySummary(
						li.entry.kind.type,
						li.entry.kind.data,
					);

					const childNode = makeNode(
						liId,
						LOOP_PADDING_X,
						childRelY,
						liName,
						liSummary,
						li.entry.kind.type,
						liStatus,
						{ duration: liDuration },
					);
					(childNode as XYNode & { parentId: string }).parentId =
						loopNodeId;
					(childNode as XYNode & { extent: string }).extent =
						"parent";
					nodes.push(childNode);

					if (loopLastChildId) {
						edges.push({
							id: `e-${loopLastChildId}-${liId}`,
							source: loopLastChildId,
							target: liId,
						});
					}

					loopLastChildId = liId;
					childRelY += NODE_HEIGHT + NODE_GAP_Y;
				}
			}

			// Advance currentY past the entire group container.
			currentY += loopHeight + NODE_GAP_Y;

			// The next node connects from the loop group's source handle (bottom).
			prevNodeId = loopLastChildId || loopNodeId;
			prevCompletedAt = completedAt;
		} else if (entryType === "join" || entryType === "race") {
			// Parallel branches: header node, then branch group containers side by side.
			const headerNodeId = `header-${item.entry.id}`;
			nodes.push(
				makeNode(
					headerNodeId,
					centerX,
					currentY,
					name,
					summary,
					entryType,
					status,
					{ duration },
				),
			);

			connectToTarget(headerNodeId, startedAt);

			currentY += NODE_HEIGHT + NODE_GAP_Y;

			const branchData =
				entryType === "join"
					? (item.entry.kind.data as JoinEntry)
					: (item.entry.kind.data as RaceEntry);
			const branchNames = Object.keys(branchData.branches);
			const nestedItems = nestedByParent.get(item.key) || [];

			// Calculate per-branch child counts and group heights.
			const branchGroupWidth = NODE_WIDTH + 2 * LOOP_PADDING_X;
			const branchInfos: {
				name: string;
				items: HistoryItem[];
				childCount: number;
				groupHeight: number;
			}[] = [];

			for (const branchName of branchNames) {
				const branchItems = nestedItems.filter((ni) =>
					ni.key.includes(`/${branchName}/`),
				);
				branchItems.sort((a, b) => {
					const aLoc = a.entry.location[
						a.entry.location.length - 1
					] as number;
					const bLoc = b.entry.location[
						b.entry.location.length - 1
					] as number;
					return aLoc - bLoc;
				});

				const childCount = branchItems.length;
				const groupHeight =
					childCount > 0
						? LOOP_HEADER_HEIGHT +
							childCount * NODE_HEIGHT +
							(childCount - 1) * NODE_GAP_Y +
							LOOP_PADDING_BOTTOM
						: LOOP_HEADER_HEIGHT + LOOP_PADDING_BOTTOM;

				branchInfos.push({
					name: branchName,
					items: branchItems,
					childCount,
					groupHeight,
				});
			}

			// Find the tallest branch to align all groups.
			const maxGroupHeight = Math.max(
				...branchInfos.map((b) => b.groupHeight),
			);

			// Layout branch groups side by side.
			const totalWidth =
				branchNames.length * branchGroupWidth +
				(branchNames.length - 1) * BRANCH_GAP_X;
			const startX =
				centerX - totalWidth / 2 + branchGroupWidth / 2 - LOOP_PADDING_X;
			const branchStartY = currentY;

			const branchGroupNodeIds: string[] = [];

			for (let bi = 0; bi < branchInfos.length; bi++) {
				const info = branchInfos[bi];
				const branchX =
					startX + bi * (branchGroupWidth + BRANCH_GAP_X);
				const branchGroupId = `branchgroup-${item.entry.id}-${info.name}`;
				const branchStatus = branchData.branches[info.name].status;

				// Create branch group container.
				nodes.push({
					id: branchGroupId,
					type: "branchGroup",
					position: { x: branchX, y: branchStartY },
					measured: {
						width: branchGroupWidth,
						height: maxGroupHeight,
					},
					style: {
						width: branchGroupWidth,
						height: maxGroupHeight,
					},
					data: {
						label: info.name,
						entryType: entryType as "join" | "race",
						branchStatus,
					},
				} as XYBranchGroupNode);

				// Edge from header to branch group.
				edges.push({
					id: `e-${headerNodeId}-${branchGroupId}`,
					source: headerNodeId,
					target: branchGroupId,
				});

				// Place children inside the branch group.
				let childRelY = LOOP_HEADER_HEIGHT;
				let lastChildId: string | null = null;

				for (const bi2 of info.items) {
					const bStartedAt = bi2.entry.startedAt;
					const bCompletedAt = bi2.entry.completedAt;
					const bDuration =
						bStartedAt && bCompletedAt
							? bCompletedAt - bStartedAt
							: undefined;
					const bStatus: EntryStatus =
						bi2.entry.status || "completed";
					const bId = `branch-${bi2.entry.id}`;
					const bName = getDisplayName(bi2.key);
					const bSummary = getEntrySummary(
						bi2.entry.kind.type,
						bi2.entry.kind.data,
					);

					const childNode = makeNode(
						bId,
						LOOP_PADDING_X,
						childRelY,
						bName,
						bSummary,
						bi2.entry.kind.type,
						bStatus,
						{ duration: bDuration },
					);
					(childNode as XYNode & { parentId: string }).parentId =
						branchGroupId;
					(childNode as XYNode & { extent: string }).extent =
						"parent";
					nodes.push(childNode);

					if (lastChildId) {
						edges.push({
							id: `e-${lastChildId}-${bId}`,
							source: lastChildId,
							target: bId,
						});
					}

					lastChildId = bId;
					childRelY += NODE_HEIGHT + NODE_GAP_Y;
				}

				branchGroupNodeIds.push(branchGroupId);
			}

			// Advance past the branch groups.
			currentY = branchStartY + maxGroupHeight + NODE_GAP_Y;

			// All branch groups will connect to the next node.
			prevNodeId = null;
			prevCompletedAt = completedAt;
			pendingBranchSources = branchGroupNodeIds;
		} else {
			// Regular sequential node.
			const nodeId = `node-${item.entry.id}`;
			nodes.push(
				makeNode(nodeId, centerX, currentY, name, summary, entryType, status, {
					duration,
					retryCount: item.entry.retryCount,
					error: item.entry.error,
				}),
			);

			connectToTarget(nodeId, startedAt);

			prevNodeId = nodeId;
			prevCompletedAt = completedAt;
			currentY += NODE_HEIGHT + NODE_GAP_Y;
		}
	}

	// Output meta node.
	if (history.output !== undefined && history.state === "completed") {
		const id = "meta-output";
		nodes.push(
			makeNode(
				id,
				centerX,
				currentY,
				"Output",
				getEntrySummary("output", { value: history.output }),
				"output",
				"completed",
			),
		);

		connectToTarget(id);
	}

	return { nodes, edges };
}
