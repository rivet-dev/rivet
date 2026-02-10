"use client";

import {
	faPlay,
	faRefresh,
	faClock,
	faEnvelope,
	faFlag,
	faCodeMerge,
	faBolt,
	faTrash,
	faMagnifyingGlassPlus,
	faMagnifyingGlassMinus,
	faMaximize,
	faRotateLeft,
	faCircleCheck,
	faCircleExclamation,
	faSpinnerThird,
	faArrowDown,
	faArrowUp,
	faXmark,
	Icon,
} from "@rivet-gg/icons";
import { useState, useRef, useCallback, useMemo, useEffect } from "react";
import { cn } from "@/components";
import type {
	WorkflowHistory,
	EntryKindType,
	ExtendedEntryType,
	EntryStatus,
	HistoryItem,
	LoopEntry,
	JoinEntry,
	RaceEntry,
	MessageEntry,
	RemovedEntry,
	StepEntry,
	SleepEntry,
	LoopIterationMarker,
} from "./workflow-types";

// Layout constants
const NODE_WIDTH = 200;
const NODE_HEIGHT = 52;
const NODE_HEIGHT_DETAILED = 100;
const NODE_GAP_Y = 32;
const BRANCH_GAP_X = 48;
const BRANCH_GAP_Y = 48;
const LOOP_PADDING_X = 24;
const LOOP_PADDING_Y = 20;
const ITERATION_HEADER = 28;

// Extended type for meta nodes
type MetaExtendedEntryType = EntryKindType | "input" | "output";

// Type colors - subtle with colored icon boxes
const TYPE_COLORS: Record<
	MetaExtendedEntryType,
	{ bg: string; border: string; icon: string; iconBg: string }
> = {
	step: {
		bg: "hsl(var(--card))",
		border: "hsl(var(--border))",
		icon: "#3b82f6",
		iconBg: "#3b82f615",
	},
	loop: {
		bg: "hsl(var(--card))",
		border: "hsl(var(--border))",
		icon: "#a855f7",
		iconBg: "#a855f715",
	},
	sleep: {
		bg: "hsl(var(--card))",
		border: "hsl(var(--border))",
		icon: "#f59e0b",
		iconBg: "#f59e0b15",
	},
	message: {
		bg: "hsl(var(--card))",
		border: "hsl(var(--border))",
		icon: "#10b981",
		iconBg: "#10b98115",
	},
	rollback_checkpoint: {
		bg: "hsl(var(--card))",
		border: "hsl(var(--border))",
		icon: "#ec4899",
		iconBg: "#ec489915",
	},
	join: {
		bg: "hsl(var(--card))",
		border: "hsl(var(--border))",
		icon: "#06b6d4",
		iconBg: "#06b6d415",
	},
	race: {
		bg: "hsl(var(--card))",
		border: "hsl(var(--border))",
		icon: "#ec4899",
		iconBg: "#ec489915",
	},
	removed: {
		bg: "hsl(var(--card))",
		border: "hsl(var(--border))",
		icon: "#71717a",
		iconBg: "#71717a15",
	},
	input: {
		bg: "hsl(var(--card))",
		border: "hsl(var(--border))",
		icon: "#22c55e",
		iconBg: "#22c55e15",
	},
	output: {
		bg: "hsl(var(--card))",
		border: "hsl(var(--border))",
		icon: "#22c55e",
		iconBg: "#22c55e15",
	},
};

// Icons
function TypeIcon({
	type,
	size = 14,
}: {
	type: MetaExtendedEntryType;
	size?: number;
}) {
	const color = TYPE_COLORS[type].icon;

	switch (type) {
		case "step":
			return <Icon icon={faPlay} style={{ color, fontSize: size }} />;
		case "loop":
			return <Icon icon={faRefresh} style={{ color, fontSize: size }} />;
		case "sleep":
			return <Icon icon={faClock} style={{ color, fontSize: size }} />;
		case "message":
			return <Icon icon={faEnvelope} style={{ color, fontSize: size }} />;
		case "rollback_checkpoint":
			return <Icon icon={faFlag} style={{ color, fontSize: size }} />;
		case "join":
			return <Icon icon={faCodeMerge} style={{ color, fontSize: size }} />;
		case "race":
			return <Icon icon={faBolt} style={{ color, fontSize: size }} />;
		case "removed":
			return <Icon icon={faTrash} style={{ color, fontSize: size }} />;
		case "input":
			return <Icon icon={faArrowDown} style={{ color, fontSize: size }} />;
		case "output":
			return <Icon icon={faArrowUp} style={{ color, fontSize: size }} />;
		default:
			return <Icon icon={faCircleCheck} style={{ color, fontSize: size }} />;
	}
}

// Get display name from key
function getDisplayName(key: string): string {
	const parts = key.split("/");
	let name = parts[parts.length - 1];
	// Remove iteration prefix
	name = name.replace(/^~\d+\//, "");
	return name;
}

// Get short summary of entry data
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
		case "rollback_checkpoint": {
			return "checkpoint";
		}
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

// Parsed structures
interface WorkflowNode {
	id: string;
	key: string;
	name: string;
	type: EntryKindType;
	data: unknown;
	locationIndex: number;
	status: EntryStatus;
	startedAt?: number;
	completedAt?: number;
	duration?: number;
	retryCount?: number;
	error?: string;
}

interface LayoutNode {
	node: WorkflowNode;
	x: number;
	y: number;
	gapFromPrev?: number;
}

interface LayoutConnection {
	id: string;
	x1: number;
	y1: number;
	x2: number;
	y2: number;
	type: "normal" | "branch" | "merge";
	deltaMs?: number;
}

interface LayoutLoop {
	id: string;
	label: string;
	iterations: number;
	x: number;
	y: number;
	width: number;
	height: number;
}

interface LayoutBranchGroup {
	id: string;
	type: "join" | "race";
	label: string;
	winner?: string | null;
	branches: {
		name: string;
		isWinner?: boolean;
		isCancelled?: boolean;
		x: number;
		y: number;
		width: number;
		height: number;
		nodes: LayoutNode[];
	}[];
	x: number;
	y: number;
	width: number;
	height: number;
}

// Parse workflow history
function parseAndLayout(
	history: WorkflowHistory,
	centerX: number,
	detailedMode = false,
	showAllDeltas = false,
) {
	const { history: items } = history;
	const nodeHeight = detailedMode ? NODE_HEIGHT_DETAILED : NODE_HEIGHT;
	const gapY = showAllDeltas ? NODE_GAP_Y + 20 : NODE_GAP_Y;

	// Separate top-level from nested
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

	// Sort by location
	topLevel.sort(
		(a, b) => (a.entry.location[0] as number) - (b.entry.location[0] as number),
	);

	const layoutNodes: LayoutNode[] = [];
	const connections: LayoutConnection[] = [];
	const loops: LayoutLoop[] = [];
	const branchGroups: LayoutBranchGroup[] = [];

	let currentY = 40;
	let prevNodeCenter: { x: number; y: number } | null = null;
	let prevCompletedAt: number | null = null;

	// Add input meta node if workflow has input
	if (history.input !== undefined) {
		const inputNode: WorkflowNode = {
			id: "meta-input",
			key: "input",
			name: "Input",
			type: "input" as EntryKindType,
			data: { value: history.input },
			locationIndex: -1,
			status: "completed",
		};

		layoutNodes.push({
			node: inputNode,
			x: centerX - NODE_WIDTH / 2,
			y: currentY,
		});

		prevNodeCenter = { x: centerX, y: currentY + nodeHeight };
		currentY += nodeHeight + gapY;
	}

	for (const item of topLevel) {
		const entryType = item.entry.kind.type;
		const startedAt = item.entry.startedAt;
		const completedAt = item.entry.completedAt;
		const duration =
			startedAt && completedAt ? completedAt - startedAt : undefined;

		// Determine status - use explicit status if provided, otherwise infer from timestamps
		const status: EntryStatus =
			item.entry.status ||
			(completedAt ? "completed" : startedAt ? "running" : "pending");

		const node: WorkflowNode = {
			id: item.entry.id,
			key: item.key,
			name: getDisplayName(item.key),
			type: entryType,
			data: item.entry.kind.data,
			locationIndex: item.entry.location[0] as number,
			status,
			startedAt,
			completedAt,
			duration,
			retryCount: item.entry.retryCount,
			error: item.entry.error,
		};

		if (entryType === "loop") {
			// Process loop with iterations
			const loopData = item.entry.kind.data as LoopEntry;
			const loopItems = nestedByParent.get(item.key) || [];

			// Group by iteration
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

			// Calculate loop dimensions
			let loopContentHeight = 0;
			const iterationLayouts: {
				iteration: number;
				nodes: LayoutNode[];
				height: number;
			}[] = [];

			for (const [iterNum, iterItems] of sortedIterations) {
				iterItems.sort((a, b) => {
					const aLoc = a.entry.location[
						a.entry.location.length - 1
					] as number;
					const bLoc = b.entry.location[
						b.entry.location.length - 1
					] as number;
					return aLoc - bLoc;
				});

				const iterNodes: LayoutNode[] = [];
				let nodeY = 0;
				for (const li of iterItems) {
					const iterStartedAt = li.entry.startedAt;
					const iterCompletedAt = li.entry.completedAt;
					const iterDuration =
						iterStartedAt && iterCompletedAt
							? iterCompletedAt - iterStartedAt
							: undefined;

					const iterNode: WorkflowNode = {
						id: li.entry.id,
						key: li.key,
						name: getDisplayName(li.key),
						type: li.entry.kind.type,
						data: li.entry.kind.data,
						locationIndex: li.entry.location[
							li.entry.location.length - 1
						] as number,
						status: li.entry.status || "completed",
						startedAt: iterStartedAt,
						completedAt: iterCompletedAt,
						duration: iterDuration,
					};
					iterNodes.push({ node: iterNode, x: 0, y: nodeY });
					nodeY += nodeHeight + NODE_GAP_Y;
				}

				const iterHeight =
					iterItems.length > 0
						? iterItems.length * nodeHeight +
							(iterItems.length - 1) * NODE_GAP_Y
						: 0;
				iterationLayouts.push({
					iteration: iterNum,
					nodes: iterNodes,
					height: iterHeight,
				});
				loopContentHeight += iterHeight + ITERATION_HEADER + 16;
			}

			if (iterationLayouts.length === 0) {
				loopContentHeight = 60;
			}

			const loopHeight = loopContentHeight + LOOP_PADDING_Y * 2;
			const loopWidth = NODE_WIDTH + LOOP_PADDING_X * 2;
			const loopX = centerX - loopWidth / 2;
			const loopY = currentY;

			// Connection from previous
			if (prevNodeCenter) {
				connections.push({
					id: `conn-to-loop-${item.entry.id}`,
					x1: prevNodeCenter.x,
					y1: prevNodeCenter.y,
					x2: centerX,
					y2: loopY,
					type: "normal",
				});
			}

			loops.push({
				id: item.entry.id,
				label: node.name,
				iterations: loopData.iteration,
				x: loopX,
				y: loopY,
				width: loopWidth,
				height: loopHeight,
			});

			// Position iteration nodes
			let iterY = loopY + LOOP_PADDING_Y;
			let prevIterLastNode: LayoutNode | null = null;

			for (const { iteration, nodes: iterNodes, height } of iterationLayouts) {
				iterY += ITERATION_HEADER;
				for (let i = 0; i < iterNodes.length; i++) {
					const ln = iterNodes[i];
					ln.x = centerX - NODE_WIDTH / 2;
					ln.y = iterY + ln.y;
					layoutNodes.push(ln);

					// Connect within iteration
					if (i > 0) {
						const prev = iterNodes[i - 1];
						const prevCompletedAtTs = prev.node.completedAt;
						const currStartedAtTs = ln.node.startedAt;
						const deltaMs =
							prevCompletedAtTs && currStartedAtTs
								? currStartedAtTs - prevCompletedAtTs
								: undefined;
						connections.push({
							id: `conn-iter-${iteration}-${i}`,
							x1: centerX,
							y1: prev.y + nodeHeight,
							x2: centerX,
							y2: ln.y,
							type: "normal",
							deltaMs,
						});
					} else if (prevIterLastNode) {
						// Connect first node of this iteration to last node of previous iteration
						const prevCompletedAtTs = prevIterLastNode.node.completedAt;
						const currStartedAtTs = ln.node.startedAt;
						const deltaMs =
							prevCompletedAtTs && currStartedAtTs
								? currStartedAtTs - prevCompletedAtTs
								: undefined;
						connections.push({
							id: `conn-iter-bridge-${iteration}`,
							x1: centerX,
							y1: prevIterLastNode.y + nodeHeight,
							x2: centerX,
							y2: ln.y,
							type: "normal",
							deltaMs,
						});
					}
				}

				// Track last node of this iteration for bridging
				if (iterNodes.length > 0) {
					prevIterLastNode = iterNodes[iterNodes.length - 1];
				}

				iterY += height + 16;
			}

			currentY = loopY + loopHeight + NODE_GAP_Y;
			prevNodeCenter = { x: centerX, y: loopY + loopHeight };
			prevCompletedAt = completedAt ?? null;
		} else if (entryType === "join" || entryType === "race") {
			// Parallel branches
			const branchData =
				entryType === "join"
					? (item.entry.kind.data as JoinEntry)
					: (item.entry.kind.data as RaceEntry);
			const branchNames = Object.keys(branchData.branches);
			const winner =
				entryType === "race" ? (branchData as RaceEntry).winner : null;

			// Add header node for the join/race
			const headerNode: LayoutNode = {
				node,
				x: centerX - NODE_WIDTH / 2,
				y: currentY,
			};
			layoutNodes.push(headerNode);

			if (prevNodeCenter) {
				connections.push({
					id: `conn-to-${node.id}`,
					x1: prevNodeCenter.x,
					y1: prevNodeCenter.y,
					x2: centerX,
					y2: currentY,
					type: "normal",
				});
			}

			const headerBottom = currentY + nodeHeight;

			// First pass: check if any branch has a significant delta from header
			let maxHeaderDelta = 0;
			for (const branchName of branchNames) {
				const branchItems = (nestedByParent.get(item.key) || []).filter(
					(bi) => bi.key.includes(`/${branchName}/`),
				);
				if (branchItems.length > 0) {
					const firstItem = branchItems.reduce((min, bi) => {
						const loc = bi.entry.location[
							bi.entry.location.length - 1
						] as number;
						const minLoc = min.entry.location[
							min.entry.location.length - 1
						] as number;
						return loc < minLoc ? bi : min;
					}, branchItems[0]);
					const delta =
						completedAt && firstItem.entry.startedAt
							? firstItem.entry.startedAt - completedAt
							: 0;
					maxHeaderDelta = Math.max(maxHeaderDelta, delta);
				}
			}

			// Add extra space if there's a significant delta to display
			const deltaSpace = maxHeaderDelta >= 500 ? 24 : 0;
			currentY = headerBottom + BRANCH_GAP_Y + deltaSpace;

			// Calculate branch layouts
			const branchLayouts: LayoutBranchGroup["branches"] = [];
			let maxBranchHeight = 0;

			for (const branchName of branchNames) {
				const branchItems = (nestedByParent.get(item.key) || []).filter(
					(bi) => bi.key.includes(`/${branchName}/`),
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

				const branchNodes: LayoutNode[] = [];
				let nodeY = 0;
				for (const bi of branchItems) {
					const bnStartedAt = bi.entry.startedAt;
					const bnCompletedAt = bi.entry.completedAt;
					const bnDuration =
						bnStartedAt && bnCompletedAt
							? bnCompletedAt - bnStartedAt
							: undefined;

					const bn: WorkflowNode = {
						id: bi.entry.id,
						key: bi.key,
						name: getDisplayName(bi.key),
						type: bi.entry.kind.type,
						data: bi.entry.kind.data,
						locationIndex: bi.entry.location[
							bi.entry.location.length - 1
						] as number,
						status: bi.entry.status || "completed",
						startedAt: bnStartedAt,
						completedAt: bnCompletedAt,
						duration: bnDuration,
					};
					branchNodes.push({ node: bn, x: 0, y: nodeY });
					nodeY += nodeHeight + NODE_GAP_Y;
				}

				const branchHeight =
					branchItems.length > 0
						? branchItems.length * nodeHeight +
							(branchItems.length - 1) * NODE_GAP_Y +
							40
						: 60;
				maxBranchHeight = Math.max(maxBranchHeight, branchHeight);

				branchLayouts.push({
					name: branchName,
					isWinner: branchName === winner,
					isCancelled:
						entryType === "race" && winner !== null && branchName !== winner,
					x: 0,
					y: 0,
					width: NODE_WIDTH,
					height: branchHeight,
					nodes: branchNodes,
				});
			}

			// Position branches horizontally
			const containerWidth = NODE_WIDTH + 40;
			const totalWidth =
				branchLayouts.length * containerWidth +
				(branchLayouts.length - 1) * BRANCH_GAP_X;
			let branchX = centerX - totalWidth / 2;

			for (const branch of branchLayouts) {
				branch.x = branchX;
				branch.y = currentY;

				// Position nodes within branch
				const branchLabelOffset = 48;
				const branchPaddingX = 20;
				const branchCenterX = branchX + branchPaddingX + NODE_WIDTH / 2;

				for (let i = 0; i < branch.nodes.length; i++) {
					const ln = branch.nodes[i];
					ln.x = branchX + branchPaddingX;
					ln.y = currentY + branchLabelOffset + ln.y;
					layoutNodes.push(ln);

					// Connect within branch
					if (i > 0) {
						const prev = branch.nodes[i - 1];
						const prevCompletedAtLocal = prev.node.completedAt;
						const currStartedAt = ln.node.startedAt;
						const deltaMs =
							prevCompletedAtLocal && currStartedAt
								? currStartedAt - prevCompletedAtLocal
								: undefined;
						connections.push({
							id: `conn-branch-${branch.name}-${i}`,
							x1: branchCenterX,
							y1: prev.y + nodeHeight,
							x2: branchCenterX,
							y2: ln.y,
							type: "normal",
							deltaMs,
						});
					}
				}

				// Connection from header to first node in branch
				const firstBranchNode = branch.nodes[0];
				const headerToFirstDelta =
					completedAt && firstBranchNode?.node.startedAt
						? firstBranchNode.node.startedAt - completedAt
						: undefined;
				connections.push({
					id: `conn-to-branch-${branch.name}`,
					x1: centerX,
					y1: headerBottom,
					x2: branchCenterX,
					y2: firstBranchNode
						? firstBranchNode.y
						: currentY + branchLabelOffset,
					type: "branch",
					deltaMs: headerToFirstDelta,
				});

				branchX += containerWidth + BRANCH_GAP_X;
			}

			// Container height = branch.height + 48 (label offset) + 20 (bottom padding)
			const containerPadding = 48 + 20;

			branchGroups.push({
				id: item.entry.id,
				type: entryType,
				label: getDisplayName(item.key),
				winner: entryType === "race" ? winner : undefined,
				branches: branchLayouts,
				x: centerX - totalWidth / 2,
				y: currentY,
				width: totalWidth,
				height: maxBranchHeight + containerPadding,
			});

			// Merge connections
			const mergeY = currentY + maxBranchHeight + containerPadding + BRANCH_GAP_Y;
			for (const branch of branchLayouts) {
				if (!branch.isCancelled) {
					const lastBranchNode = branch.nodes[branch.nodes.length - 1];
					const lastNodeCompletedAt = lastBranchNode?.node.completedAt;
					const branchPaddingX = 20;
					const branchCenterX = branch.x + branchPaddingX + NODE_WIDTH / 2;
					const containerBottom = branch.y + branch.height + containerPadding;
					connections.push({
						id: `conn-merge-${branch.name}`,
						x1: branchCenterX,
						y1: containerBottom,
						x2: centerX,
						y2: mergeY,
						type: "merge",
						deltaMs:
							lastNodeCompletedAt && completedAt
								? completedAt - lastNodeCompletedAt
								: undefined,
					});
				}
			}

			currentY = mergeY;
			prevNodeCenter = null;
			prevCompletedAt = completedAt ?? null;
		} else {
			// Regular sequential node
			let gapFromPrev: number | undefined;
			if (prevCompletedAt && startedAt) {
				gapFromPrev = startedAt - prevCompletedAt;
			}

			const layoutNode: LayoutNode = {
				node,
				x: centerX - NODE_WIDTH / 2,
				y: currentY,
				gapFromPrev,
			};
			layoutNodes.push(layoutNode);

			if (prevNodeCenter) {
				connections.push({
					id: `conn-to-${node.id}`,
					x1: prevNodeCenter.x,
					y1: prevNodeCenter.y,
					x2: centerX,
					y2: currentY,
					type: "normal",
					deltaMs: gapFromPrev,
				});
			}

			prevNodeCenter = { x: centerX, y: currentY + nodeHeight };
			prevCompletedAt = completedAt ?? null;
			currentY += nodeHeight + gapY;
		}
	}

	// Calculate total width from all positioned elements
	let maxX = centerX + NODE_WIDTH / 2;

	// Add output meta node if workflow has output (only for completed workflows)
	if (history.output !== undefined && history.state === "completed") {
		const outputNode: WorkflowNode = {
			id: "meta-output",
			key: "output",
			name: "Output",
			type: "output" as EntryKindType,
			data: { value: history.output },
			locationIndex: 9999,
			status: "completed",
		};

		// Add connection from last node to output
		if (prevNodeCenter) {
			connections.push({
				id: "conn-to-output",
				x1: prevNodeCenter.x,
				y1: prevNodeCenter.y,
				x2: centerX,
				y2: currentY,
				type: "normal",
			});
		}

		layoutNodes.push({
			node: outputNode,
			x: centerX - NODE_WIDTH / 2,
			y: currentY,
		});

		currentY += nodeHeight + gapY;
	}

	for (const ln of layoutNodes) {
		maxX = Math.max(maxX, ln.x + NODE_WIDTH);
	}
	for (const loop of loops) {
		maxX = Math.max(maxX, loop.x + loop.width);
	}
	for (const group of branchGroups) {
		for (const branch of group.branches) {
			maxX = Math.max(maxX, branch.x + branch.width);
		}
	}

	return {
		nodes: layoutNodes,
		connections,
		loops,
		branchGroups,
		totalWidth: maxX + 60,
		totalHeight: currentY + 60,
	};
}

// Format duration for display
function formatDuration(ms: number): string {
	if (ms < 1000) return `${ms}ms`;
	if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
	if (ms < 3600000) return `${(ms / 60000).toFixed(1)}m`;
	return `${(ms / 3600000).toFixed(1)}h`;
}

// SVG Node - reports hover to parent for popover rendering
function SVGNode({
	node,
	x,
	y,
	selected,
	onSelect,
	onHover,
	detailedMode,
}: {
	node: WorkflowNode;
	x: number;
	y: number;
	selected: boolean;
	onSelect: (node: WorkflowNode) => void;
	onHover: (node: WorkflowNode | null, x: number, y: number) => void;
	gapFromPrev?: number;
	detailedMode?: boolean;
}) {
	const colors = TYPE_COLORS[node.type as MetaExtendedEntryType];
	const summary = getEntrySummary(node.type, node.data);
	const duration = node.duration;
	const height = detailedMode ? NODE_HEIGHT_DETAILED : NODE_HEIGHT;
	const isRunning = node.status === "running";
	const isFailed = node.status === "failed";
	const isRetrying = node.status === "retrying";

	// Get data preview for detailed mode
	const dataPreview = detailedMode
		? JSON.stringify(node.data, null, 2).slice(0, 120)
		: "";

	return (
		<g transform={`translate(${x}, ${y})`}>
			{/* biome-ignore lint/a11y/noStaticElementInteractions: SVG node for workflow visualization */}
			<g
				onClick={(e) => {
					e.stopPropagation();
					onSelect(node);
				}}
				onMouseEnter={() => onHover(node, x, y)}
				onMouseLeave={() => onHover(null, 0, 0)}
				className="cursor-pointer"
			>
				{/* Card background */}
				<rect
					x={0}
					y={0}
					width={NODE_WIDTH}
					height={height}
					rx={10}
					fill={colors.bg}
					stroke={selected ? "#52525b" : isFailed ? "#ef4444" : colors.border}
					strokeWidth={isFailed ? 2 : 1}
					className="transition-all duration-150"
				/>
				{/* Retry count badge */}
				{node.retryCount && node.retryCount > 0 && (
					<g>
						<rect
							x={NODE_WIDTH - 32}
							y={-8}
							width={28}
							height={16}
							rx={4}
							fill="#18181b"
							stroke={isFailed ? "#ef4444" : "#f59e0b"}
							strokeWidth={1}
						/>
						<text
							x={NODE_WIDTH - 18}
							y={0}
							fill={isFailed ? "#ef4444" : "#f59e0b"}
							fontSize={9}
							fontFamily="system-ui"
							textAnchor="middle"
							dominantBaseline="middle"
						>
							x{node.retryCount}
						</text>
					</g>
				)}
				{/* Duration - bottom right inside card (only when not running/retrying) */}
				{duration !== undefined &&
					!node.retryCount &&
					!isRunning &&
					!isRetrying && (
						<text
							x={NODE_WIDTH - 12}
							y={height - 12}
							fill="#71717a"
							fontSize={10}
							fontFamily="system-ui"
							textAnchor="end"
							dominantBaseline="middle"
						>
							{formatDuration(duration)}
						</text>
					)}
				{/* Status indicator - right side for running/retrying */}
				{(isRunning || isRetrying) && (
					<foreignObject x={NODE_WIDTH - 36} y={14} width={24} height={24}>
						<Icon
							icon={faSpinnerThird}
							className="animate-spin text-muted-foreground"
							style={{ fontSize: 20 }}
						/>
					</foreignObject>
				)}
				{isFailed && (
					<foreignObject x={NODE_WIDTH - 36} y={14} width={24} height={24}>
						<Icon
							icon={faCircleExclamation}
							className="text-destructive"
							style={{ fontSize: 20 }}
						/>
					</foreignObject>
				)}
				{/* Icon box with color */}
				<rect
					x={10}
					y={10}
					width={32}
					height={32}
					rx={8}
					fill={colors.iconBg}
					stroke={colors.icon}
					strokeWidth={1}
					strokeOpacity={0.3}
				/>
				<foreignObject x={19} y={19} width={14} height={14}>
					<TypeIcon type={node.type as MetaExtendedEntryType} size={14} />
				</foreignObject>
				{/* Text */}
				<text
					x={52}
					y={26}
					fill="hsl(var(--foreground))"
					fontSize={12}
					fontWeight={500}
					fontFamily="system-ui"
				>
					{node.name.length > 18 ? `${node.name.slice(0, 18)}...` : node.name}
				</text>
				<text
					x={52}
					y={40}
					fill="hsl(var(--muted-foreground))"
					fontSize={10}
					fontFamily="system-ui"
				>
					{summary}
				</text>
				{/* Detailed mode: show data preview */}
				{detailedMode && (
					<foreignObject x={10} y={50} width={NODE_WIDTH - 20} height={44}>
						<div
							style={{
								fontFamily: "ui-monospace, monospace",
								fontSize: 9,
								color: "#52525b",
								overflow: "hidden",
								lineHeight: 1.3,
								whiteSpace: "pre-wrap",
								wordBreak: "break-all",
							}}
						>
							{dataPreview}
							{dataPreview.length >= 120 ? "..." : ""}
						</div>
					</foreignObject>
				)}
			</g>
		</g>
	);
}

// Connection with spacing for arrowhead and delta display on hover
function Connection({
	x1,
	y1,
	x2,
	y2,
	type,
	deltaMs,
	showAllDeltas,
}: LayoutConnection & { showAllDeltas?: boolean }) {
	const [isHovered, setIsHovered] = useState(false);

	// Add spacing at the end for the arrowhead
	const arrowGap = 8;
	const adjustedY2 = y2 - arrowGap;

	const radius = 8;

	// Show delta: always if >= 500ms, on hover for smaller, or always if showAllDeltas
	const isSignificantDelta = deltaMs !== undefined && deltaMs >= 500;
	const shouldShowDelta =
		deltaMs !== undefined && (isSignificantDelta || isHovered || showAllDeltas);

	// Build path based on connection type
	let path: string;
	if (type === "normal" || x1 === x2) {
		// Straight vertical line
		path = `M ${x1} ${y1 + 4} L ${x2} ${adjustedY2}`;
	} else {
		// Branching path
		const startY = y1 + 4;
		const midY = startY + 20;
		const endY = adjustedY2;

		const goingRight = x2 > x1;
		const hDir = goingRight ? 1 : -1;

		path = `M ${x1} ${startY}
            L ${x1} ${midY - radius}
            Q ${x1} ${midY} ${x1 + radius * hDir} ${midY}
            L ${x2 - radius * hDir} ${midY}
            Q ${x2} ${midY} ${x2} ${midY + radius}
            L ${x2} ${endY}`;
	}

	return (
		// biome-ignore lint/a11y/noStaticElementInteractions: SVG connection for workflow visualization
		<g
			onMouseEnter={() => setIsHovered(true)}
			onMouseLeave={() => setIsHovered(false)}
			style={{ cursor: deltaMs !== undefined ? "pointer" : "default" }}
		>
			{/* Invisible wider path for easier hover targeting */}
			<path
				d={path}
				fill="none"
				stroke="transparent"
				strokeWidth={12}
				strokeLinecap="round"
				strokeLinejoin="round"
			/>
			{/* Visible path */}
			<path
				d={path}
				fill="none"
				stroke={
					isHovered && deltaMs !== undefined
						? "hsl(var(--muted-foreground))"
						: "hsl(var(--border))"
				}
				strokeWidth={1.5}
				strokeLinecap="round"
				strokeLinejoin="round"
				markerEnd="url(#arrowhead)"
				className="transition-colors duration-150"
			/>
			{shouldShowDelta &&
				(() => {
					let textX: number;
					let textY: number;

					if (type === "normal" || x1 === x2) {
						textX = x1 + 12;
						textY = (y1 + y2) / 2;
					} else {
						const midY = y1 + 4 + 20;
						textX = x2 + 12;
						textY = (midY + y2) / 2;
					}

					return (
						<text
							x={textX}
							y={textY}
							fill="hsl(var(--muted-foreground))"
							fontSize={10}
							fontFamily="system-ui"
							dominantBaseline="middle"
						>
							{deltaMs !== undefined && formatDuration(deltaMs)} later
						</text>
					);
				})()}
		</g>
	);
}

// Main component
export function WorkflowVisualizer({
	workflow,
}: {
	workflow: WorkflowHistory;
}) {
	const [transform, setTransform] = useState({ x: 60, y: 60, scale: 1 });
	const [isPanning, setIsPanning] = useState(false);
	const [panStart, setPanStart] = useState({ x: 0, y: 0 });
	const [selectedNode, setSelectedNode] = useState<WorkflowNode | null>(null);
	const [hoveredNode, setHoveredNode] = useState<{
		node: WorkflowNode;
		x: number;
		y: number;
	} | null>(null);
	const [detailedMode, setDetailedMode] = useState(false);

	const containerRef = useRef<HTMLDivElement>(null);
	const [hasInitialized, setHasInitialized] = useState(false);
	const [showAllDeltas, setShowAllDeltas] = useState(false);

	const handleNodeHover = useCallback(
		(node: WorkflowNode | null, x: number, y: number) => {
			if (node) {
				setHoveredNode({ node, x, y });
			} else {
				setHoveredNode(null);
			}
		},
		[],
	);

	const layout = useMemo(
		() => parseAndLayout(workflow, 350, detailedMode, showAllDeltas),
		[workflow, detailedMode, showAllDeltas],
	);

	// Auto-fit the workflow on initial render
	useEffect(() => {
		if (hasInitialized || !containerRef.current) return;

		const container = containerRef.current;
		const containerWidth = container.clientWidth;
		const containerHeight = container.clientHeight;

		const padding = 80;
		const contentWidth = layout.totalWidth;
		const contentHeight = layout.totalHeight;

		// Calculate scale to fit content with padding
		const scaleX = (containerWidth - padding * 2) / contentWidth;
		const scaleY = (containerHeight - padding * 2) / contentHeight;

		// Use the smaller scale to fit both dimensions, max out at 1 for short workflows
		const scale = Math.min(scaleX, scaleY, 1);

		// Center the content
		const scaledWidth = contentWidth * scale;
		const scaledHeight = contentHeight * scale;
		const x = (containerWidth - scaledWidth) / 2;
		const y = (containerHeight - scaledHeight) / 2;

		setTransform({ x, y, scale });
		setHasInitialized(true);
	}, [layout, hasInitialized]);

	const handleMouseDown = useCallback(
		(e: React.MouseEvent) => {
			if (e.button === 0 || e.button === 1) {
				e.preventDefault();
				setIsPanning(true);
				setPanStart({ x: e.clientX - transform.x, y: e.clientY - transform.y });
			}
		},
		[transform],
	);

	const handleMouseMove = useCallback(
		(e: React.MouseEvent) => {
			if (isPanning) {
				setTransform((t) => ({
					...t,
					x: e.clientX - panStart.x,
					y: e.clientY - panStart.y,
				}));
			}
		},
		[isPanning, panStart],
	);

	const handleMouseUp = useCallback(() => setIsPanning(false), []);

	const handleWheel = useCallback((e: React.WheelEvent) => {
		e.preventDefault();

		if (e.ctrlKey || e.metaKey) {
			const rect = containerRef.current?.getBoundingClientRect();
			if (!rect) return;

			const cursorX = e.clientX - rect.left;
			const cursorY = e.clientY - rect.top;

			const delta = e.deltaY > 0 ? 0.9 : 1.1;

			setTransform((t) => {
				const newScale = Math.min(Math.max(t.scale * delta, 0.25), 2);
				const scaleFactor = newScale / t.scale;

				const newX = cursorX - (cursorX - t.x) * scaleFactor;
				const newY = cursorY - (cursorY - t.y) * scaleFactor;

				return { x: newX, y: newY, scale: newScale };
			});
		} else {
			setTransform((t) => ({
				...t,
				x: t.x - e.deltaX,
				y: t.y - e.deltaY,
			}));
		}
	}, []);

	const zoomIn = () =>
		setTransform((t) => ({ ...t, scale: Math.min(t.scale * 1.2, 2) }));
	const zoomOut = () =>
		setTransform((t) => ({ ...t, scale: Math.max(t.scale / 1.2, 0.25) }));
	const resetView = () => setTransform({ x: 60, y: 60, scale: 1 });
	const fitView = () => {
		if (containerRef.current) {
			const { width, height } = containerRef.current.getBoundingClientRect();
			const scale = Math.min(width / 800, height / layout.totalHeight, 1) * 0.85;
			setTransform({ x: 60, y: 60, scale });
		}
	};

	return (
		<div className="flex h-full w-full flex-col bg-background">
			<div className="relative flex min-h-0 flex-1">
				{/* biome-ignore lint/a11y/noStaticElementInteractions: Canvas for panning/zooming workflow visualization */}
				<div
					ref={containerRef}
					className="relative flex-1 overflow-hidden"
					onMouseDown={handleMouseDown}
					onMouseMove={handleMouseMove}
					onMouseUp={handleMouseUp}
					onMouseLeave={handleMouseUp}
					onWheel={handleWheel}
					style={{ cursor: isPanning ? "grabbing" : "grab" }}
				>
					{/* Dot grid */}
					<svg
						className="absolute inset-0 h-full w-full"
						style={{ zIndex: 0 }}
						aria-hidden="true"
					>
						<defs>
							<pattern
								id="dotGrid"
								width="20"
								height="20"
								patternUnits="userSpaceOnUse"
							>
								<circle
									cx="10"
									cy="10"
									r="0.75"
									fill="hsl(var(--border))"
								/>
							</pattern>
						</defs>
						<rect width="100%" height="100%" fill="url(#dotGrid)" />
					</svg>

					{/* Workflow */}
					<div
						style={{
							position: "absolute",
							left: 0,
							top: 0,
							transform: `translate(${transform.x}px, ${transform.y}px) scale(${transform.scale})`,
							transformOrigin: "0 0",
						}}
					>
						<svg
							width={layout.totalWidth + 200}
							height={layout.totalHeight + 200}
							style={{ overflow: "visible" }}
							role="img"
							aria-label="Workflow visualization"
						>
							<defs>
								<marker
									id="arrowhead"
									markerWidth="12"
									markerHeight="12"
									refX="6"
									refY="6"
									orient="auto"
									markerUnits="userSpaceOnUse"
								>
									<path
										d="M 1 2 L 6 6 L 1 10"
										fill="none"
										stroke="hsl(var(--border))"
										strokeWidth="1.5"
										strokeLinecap="round"
										strokeLinejoin="round"
									/>
								</marker>
							</defs>

							{/* Connections */}
							{layout.connections.map((conn) => (
								<Connection
									key={conn.id}
									{...conn}
									showAllDeltas={showAllDeltas}
								/>
							))}

							{/* Loop containers */}
							{layout.loops.map((loop) => (
								<g key={loop.id}>
									<rect
										x={loop.x}
										y={loop.y}
										width={loop.width}
										height={loop.height}
										rx={12}
										fill="#a855f708"
										stroke="#a855f725"
										strokeWidth={1}
										pointerEvents="none"
									/>
									<rect
										x={loop.x + 24}
										y={loop.y + 12}
										width={150}
										height={28}
										rx={6}
										fill="hsl(var(--card))"
										stroke="#a855f730"
										strokeWidth={1}
									/>
									<foreignObject
										x={loop.x + 32}
										y={loop.y + 19}
										width={14}
										height={14}
									>
										<Icon
											icon={faRefresh}
											style={{ color: "#a855f7", fontSize: 14 }}
										/>
									</foreignObject>
									<text
										x={loop.x + 52}
										y={loop.y + 26}
										fill="hsl(var(--muted-foreground))"
										fontSize={11}
										fontWeight={500}
										fontFamily="system-ui"
										dominantBaseline="middle"
									>
										{loop.label}{" "}
										<tspan fill="hsl(var(--muted-foreground))">
											({loop.iterations}x)
										</tspan>
									</text>
								</g>
							))}

							{/* Branch containers */}
							{layout.branchGroups.map((group) => {
								const baseColor =
									group.type === "join" ? "#06b6d4" : "#ec4899";
								const iconDef = group.type === "join" ? faCodeMerge : faBolt;
								return (
									<g key={group.id}>
										{group.branches.map((branch) => {
											const branchColor = branch.isWinner
												? "#10b981"
												: branch.isCancelled
													? "#ef4444"
													: baseColor;
											const containerX = branch.x;
											const containerY = branch.y;
											const containerWidth = branch.width + 40;
											const containerHeight = branch.height + 48 + 20;
											const containerCenterX = containerX + containerWidth / 2;
											return (
												<g key={branch.name}>
													<rect
														x={containerX}
														y={containerY}
														width={containerWidth}
														height={containerHeight}
														rx={12}
														fill={`${branchColor}08`}
														stroke={`${branchColor}25`}
														strokeWidth={1}
														pointerEvents="none"
													/>
													<rect
														x={containerX + 20}
														y={containerY + 12}
														width={150}
														height={28}
														rx={6}
														fill="hsl(var(--card))"
														stroke={`${branchColor}30`}
														strokeWidth={1}
													/>
													<foreignObject
														x={containerX + 28}
														y={containerY + 19}
														width={14}
														height={14}
													>
														<Icon
															icon={iconDef}
															style={{ color: branchColor, fontSize: 14 }}
														/>
													</foreignObject>
													<text
														x={containerX + 48}
														y={containerY + 26}
														fill="hsl(var(--muted-foreground))"
														fontSize={11}
														fontWeight={500}
														fontFamily="system-ui"
														dominantBaseline="middle"
													>
														{branch.name}
													</text>
													{branch.isCancelled && (
														<g
															transform={`translate(${containerCenterX}, ${containerY + containerHeight})`}
														>
															<line
																x1={0}
																y1={0}
																x2={0}
																y2={20}
																stroke="hsl(var(--border))"
																strokeWidth={1.5}
															/>
															<circle
																cx={0}
																cy={32}
																r={10}
																fill="hsl(var(--card))"
																stroke="#ef444450"
																strokeWidth={1}
															/>
															<line
																x1={-4}
																y1={28}
																x2={4}
																y2={36}
																stroke="#ef4444"
																strokeWidth={1.5}
																strokeLinecap="round"
															/>
															<line
																x1={4}
																y1={28}
																x2={-4}
																y2={36}
																stroke="#ef4444"
																strokeWidth={1.5}
																strokeLinecap="round"
															/>
														</g>
													)}
												</g>
											);
										})}
									</g>
								);
							})}

							{/* Nodes */}
							{layout.nodes.map(({ node, x, y, gapFromPrev }) => (
								<SVGNode
									key={node.id}
									node={node}
									x={x}
									y={y}
									selected={selectedNode?.id === node.id}
									onHover={handleNodeHover}
									onSelect={setSelectedNode}
									gapFromPrev={gapFromPrev}
									detailedMode={detailedMode}
								/>
							))}
						</svg>
					</div>

					{/* Zoom controls */}
					<div className="absolute bottom-4 right-4 flex flex-col gap-1.5 rounded-lg border border-border bg-card p-1.5 shadow-lg">
						<button
							type="button"
							onClick={zoomIn}
							className="flex h-7 w-7 items-center justify-center rounded hover:bg-secondary"
						>
							<Icon icon={faMagnifyingGlassPlus} className="text-foreground" />
						</button>
						<button
							type="button"
							onClick={zoomOut}
							className="flex h-7 w-7 items-center justify-center rounded hover:bg-secondary"
						>
							<Icon icon={faMagnifyingGlassMinus} className="text-foreground" />
						</button>
						<div className="h-px bg-border" />
						<button
							type="button"
							onClick={fitView}
							className="flex h-7 w-7 items-center justify-center rounded hover:bg-secondary"
						>
							<Icon icon={faMaximize} className="text-foreground" />
						</button>
						<button
							type="button"
							onClick={resetView}
							className="flex h-7 w-7 items-center justify-center rounded hover:bg-secondary"
						>
							<Icon icon={faRotateLeft} className="text-foreground" />
						</button>
						<div className="mt-1 text-center text-xs text-muted-foreground">
							{Math.round(transform.scale * 100)}%
						</div>
					</div>

					{/* Header */}
					<div className="absolute left-4 top-4 flex items-center gap-3 rounded-lg border border-border bg-card px-3 py-2 shadow-lg">
						<div className="flex h-7 w-7 items-center justify-center rounded bg-primary/20">
							<Icon icon={faCodeMerge} className="text-primary" />
						</div>
						<div>
							<div className="text-sm font-medium text-foreground">
								Workflow
							</div>
							<div className="text-xs text-muted-foreground">
								{workflow.workflowId.slice(0, 8)}... | {workflow.state}
							</div>
						</div>
						<div className="ml-4 h-6 w-px bg-border" />
						<label className="flex cursor-pointer items-center gap-2">
							<input
								type="checkbox"
								checked={detailedMode}
								onChange={(e) => setDetailedMode(e.target.checked)}
								className="h-3.5 w-3.5 rounded border-border bg-secondary accent-primary"
							/>
							<span className="text-xs text-muted-foreground">Detailed</span>
						</label>
						<label className="flex cursor-pointer items-center gap-2">
							<input
								type="checkbox"
								checked={showAllDeltas}
								onChange={(e) => setShowAllDeltas(e.target.checked)}
								className="h-3.5 w-3.5 rounded border-border bg-secondary accent-primary"
							/>
							<span className="text-xs text-muted-foreground">
								Show All Deltas
							</span>
						</label>
					</div>

					{/* Hover popover - rendered as HTML for proper z-index */}
					{hoveredNode && !detailedMode && (
						<div
							className="pointer-events-none absolute z-50"
							style={{
								left:
									transform.x +
									(hoveredNode.x + NODE_WIDTH + 12) * transform.scale,
								top: transform.y + (hoveredNode.y - 10) * transform.scale,
							}}
						>
							<div
								className="rounded-lg border border-border bg-card p-3 shadow-xl"
								style={{ width: 240 }}
							>
								<div className="mb-2">
									<div className="mb-0.5 text-[10px] font-medium uppercase text-muted-foreground">
										Key
									</div>
									<div className="break-all font-mono text-xs text-foreground">
										{hoveredNode.node.key}
									</div>
								</div>
								{hoveredNode.node.startedAt && (
									<div className="mb-2">
										<div className="mb-0.5 text-[10px] font-medium uppercase text-muted-foreground">
											Started
										</div>
										<div className="font-mono text-xs text-muted-foreground">
											{new Date(
												hoveredNode.node.startedAt,
											).toLocaleString()}
											.
											{String(hoveredNode.node.startedAt % 1000).padStart(
												3,
												"0",
											)}
										</div>
									</div>
								)}
								{hoveredNode.node.completedAt && (
									<div className="mb-2">
										<div className="mb-0.5 text-[10px] font-medium uppercase text-muted-foreground">
											Completed
										</div>
										<div className="font-mono text-xs text-muted-foreground">
											{new Date(
												hoveredNode.node.completedAt,
											).toLocaleString()}
											.
											{String(hoveredNode.node.completedAt % 1000).padStart(
												3,
												"0",
											)}
										</div>
									</div>
								)}
								{hoveredNode.node.error && (
									<div className="mb-2">
										<div className="mb-0.5 text-[10px] font-medium uppercase text-destructive">
											Error
										</div>
										<div className="text-xs text-destructive">
											{hoveredNode.node.error}
										</div>
									</div>
								)}
								{hoveredNode.node.retryCount &&
									hoveredNode.node.retryCount > 0 && (
										<div className="mb-2">
											<div className="mb-0.5 text-[10px] font-medium uppercase text-amber-500">
												Retries
											</div>
											<div className="text-xs text-amber-500">
												{hoveredNode.node.retryCount} attempt(s)
											</div>
										</div>
									)}
								<div>
									<div className="mb-0.5 text-[10px] font-medium uppercase text-muted-foreground">
										Data
									</div>
									<pre className="max-h-20 overflow-hidden font-mono text-[9px] text-muted-foreground">
										{JSON.stringify(hoveredNode.node.data, null, 2).slice(
											0,
											200,
										)}
										{JSON.stringify(hoveredNode.node.data, null, 2).length >
										200
											? "..."
											: ""}
									</pre>
								</div>
							</div>
						</div>
					)}
				</div>
			</div>

			{/* Bottom details panel */}
			{selectedNode && (
				<div className="border-t border-border bg-card">
					<div className="flex items-start gap-6 px-6 py-4">
						<div className="flex items-center gap-3">
							<div
								className="flex h-10 w-10 items-center justify-center rounded-lg"
								style={{
									backgroundColor:
										TYPE_COLORS[selectedNode.type as MetaExtendedEntryType]
											.iconBg,
								}}
							>
								<TypeIcon
									type={selectedNode.type as MetaExtendedEntryType}
									size={18}
								/>
							</div>
							<div>
								<div className="font-medium text-foreground">
									{selectedNode.name}
								</div>
								<div className="text-xs text-muted-foreground">
									{selectedNode.type}
								</div>
							</div>
						</div>

						<div className="h-10 w-px bg-border" />

						<div className="min-w-0 flex-1">
							<div className="mb-1 text-xs font-medium uppercase text-muted-foreground">
								Key
							</div>
							<div className="truncate rounded bg-secondary px-2 py-1 font-mono text-xs text-foreground">
								{selectedNode.key}
							</div>
						</div>

						<div className="min-w-0 flex-1">
							<div className="mb-1 text-xs font-medium uppercase text-muted-foreground">
								Data
							</div>
							<pre className="max-h-20 overflow-auto rounded bg-secondary px-2 py-1 font-mono text-xs text-foreground">
								{JSON.stringify(selectedNode.data, null, 2)}
							</pre>
						</div>

						{selectedNode.startedAt && (
							<div>
								<div className="mb-1 text-xs font-medium uppercase text-muted-foreground">
									Started
								</div>
								<div className="rounded bg-secondary px-2 py-1 font-mono text-xs text-foreground">
									{new Date(selectedNode.startedAt).toLocaleString()}
								</div>
							</div>
						)}

						{selectedNode.completedAt && (
							<div>
								<div className="mb-1 text-xs font-medium uppercase text-muted-foreground">
									Completed
								</div>
								<div className="rounded bg-secondary px-2 py-1 font-mono text-xs text-foreground">
									{new Date(selectedNode.completedAt).toLocaleString()}
								</div>
							</div>
						)}

						{selectedNode.duration !== undefined && (
							<div>
								<div className="mb-1 text-xs font-medium uppercase text-muted-foreground">
									Duration
								</div>
								<div className="rounded bg-secondary px-2 py-1 font-mono text-xs text-foreground">
									{formatDuration(selectedNode.duration)}
								</div>
							</div>
						)}

						<div>
							<div className="mb-1 text-xs font-medium uppercase text-muted-foreground">
								Status
							</div>
							<div
								className={cn(
									"rounded px-2 py-1 font-mono text-xs",
									selectedNode.status === "completed" &&
										"bg-emerald-500/10 text-emerald-500",
									selectedNode.status === "running" &&
										"bg-blue-500/10 text-blue-500",
									selectedNode.status === "failed" &&
										"bg-red-500/10 text-red-500",
									selectedNode.status === "retrying" &&
										"bg-amber-500/10 text-amber-500",
									selectedNode.status === "pending" &&
										"bg-secondary text-muted-foreground",
								)}
							>
								{selectedNode.status}
							</div>
						</div>

						{selectedNode.retryCount && selectedNode.retryCount > 0 && (
							<div>
								<div className="mb-1 text-xs font-medium uppercase text-amber-500">
									Retries
								</div>
								<div className="rounded bg-amber-500/10 px-2 py-1 font-mono text-xs text-amber-500">
									{selectedNode.retryCount}
								</div>
							</div>
						)}

						{selectedNode.error && (
							<div className="min-w-0 flex-1">
								<div className="mb-1 text-xs font-medium uppercase text-destructive">
									Error
								</div>
								<div className="truncate rounded bg-red-500/10 px-2 py-1 font-mono text-xs text-destructive">
									{selectedNode.error}
								</div>
							</div>
						)}

						<button
							type="button"
							onClick={() => setSelectedNode(null)}
							className="flex h-8 w-8 items-center justify-center rounded hover:bg-secondary"
						>
							<Icon icon={faXmark} className="text-muted-foreground" />
						</button>
					</div>
				</div>
			)}
		</div>
	);
}
