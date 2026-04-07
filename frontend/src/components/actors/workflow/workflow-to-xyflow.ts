import type { Edge, Node } from "@xyflow/react";
import type {
	EntryKindType,
	EntryStatus,
	ExtendedEntryType,
	HistoryItem,
	JoinEntry,
	Location,
	MessageEntry,
	PathSegment,
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
	type TryGroupNodeData,
	type WorkflowNodeData,
} from "./xyflow-nodes";

const NODE_GAP_Y = 48;
const BRANCH_GAP_X = 60;
const GROUP_WIDTH = NODE_WIDTH + 2 * LOOP_PADDING_X;
const TERMINATION_GAP = 24;

type XYNode = Node<WorkflowNodeData, "workflow">;
type XYLoopGroupNode = Node<LoopGroupNodeData, "loopGroup">;
type XYTryGroupNode = Node<TryGroupNodeData, "tryGroup">;
type XYBranchGroupNode = Node<BranchGroupNodeData, "branchGroup">;
type XYTerminationNode = Node<TerminationNodeData, "termination">;
type AnyXYNode =
	| XYNode
	| XYLoopGroupNode
	| XYTryGroupNode
	| XYBranchGroupNode
	| XYTerminationNode;

export interface LayoutResult {
	nodes: AnyXYNode[];
	edges: Edge[];
}

interface WorkflowLayoutOptions {
	currentStepId?: string;
	replayingEntryId?: string;
	onReplayStep?: (entryId: string) => void;
}

type WorkflowNodeInput = {
	label?: string;
	summary?: string;
	entryType: EntryKindType | "input" | "output";
	status: EntryStatus;
	handledFailure?: boolean;
	duration?: number;
	retryCount?: number;
	error?: string;
	nodeKey?: string;
	startedAt?: number;
	completedAt?: number;
	rawData?: unknown;
	name?: string;
	entryId?: string;
	onReplayStep?: (entryId: string) => void;
};

type Bounds = {
	minX: number;
	minY: number;
	maxX: number;
	maxY: number;
};

interface LayoutFragment {
	nodes: AnyXYNode[];
	edges: Edge[];
	bounds: Bounds | null;
	firstTargetId?: string;
	firstStartedAt?: number;
	outgoingSources: string[];
	lastCompletedAt?: number;
}

interface RenderTreeBase {
	id: string;
	key: string;
	name: string;
	location: Location;
	children: RenderTreeNode[];
}

interface RenderTreeEntryNode extends RenderTreeBase {
	kind: "entry";
	item: HistoryItem;
}

interface RenderTreeTryNode extends RenderTreeBase {
	kind: "try";
}

type RenderTreeNode = RenderTreeEntryNode | RenderTreeTryNode;

interface LayoutContext {
	workflow: WorkflowHistory;
	hasTryAncestor: boolean;
}

interface TreeStats {
	handledFailureNames: string[];
	unhandledFailureNames: string[];
	hasRunning: boolean;
	hasPending: boolean;
}

function getDisplayName(key: string): string {
	const parts = key.split("/");
	return parts[parts.length - 1]?.replace(/^~\d+\//, "") ?? key;
}

function getEntrySummary(
	type: ExtendedEntryType,
	data: unknown,
	options?: { handledFailure?: boolean },
): string {
	switch (type) {
		case "step": {
			const d = data as StepEntry;
			if (options?.handledFailure) return "handled error";
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
			return `${(data as { iteration: number }).iteration} iterations`;
		case "rollback_checkpoint":
			return "checkpoint";
		case "join": {
			const d = data as JoinEntry;
			const done = Object.values(d.branches).filter(
				(branch) => branch.status === "completed",
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
		case "try":
			return "protected scope";
		default:
			return "";
	}
}

function deriveEntryStatus(item: HistoryItem): EntryStatus {
	const rawStatus = item.entry.status;
	if (rawStatus) {
		return rawStatus;
	}
	if (item.entry.completedAt != null) {
		return "completed";
	}
	if (item.entry.startedAt != null) {
		return "running";
	}
	return "pending";
}

function itemToNodeData(
	item: HistoryItem,
	_workflow: WorkflowHistory,
	options: {
		handledFailure?: boolean;
		onReplayStep?: (entryId: string) => void;
	} = {},
) {
	const { id, startedAt, completedAt, kind, retryCount, error } = item.entry;
	const duration =
		startedAt != null && completedAt != null
			? completedAt - startedAt
			: undefined;
	const handledFailure =
		options?.handledFailure &&
		(kind.type === "step" || deriveEntryStatus(item) === "failed");

	return {
		name: getDisplayName(item.key),
		summary: getEntrySummary(kind.type, kind.data, {
			handledFailure,
		}),
		entryType: kind.type,
		status: deriveEntryStatus(item),
		handledFailure,
		duration,
		startedAt,
		completedAt,
		retryCount,
		error,
		rawData: kind.data,
		nodeKey: item.key,
		entryId: id,
		onReplayStep: options.onReplayStep,
	};
}

function comparePathSegments(a: PathSegment, b: PathSegment): number {
	if (typeof a === "number" && typeof b === "number") {
		return a - b;
	}
	if (typeof a === "number") {
		return -1;
	}
	if (typeof b === "number") {
		return 1;
	}
	if (a.loop !== b.loop) {
		return a.loop - b.loop;
	}
	return a.iteration - b.iteration;
}

function compareLocations(a: Location, b: Location): number {
	const shared = Math.min(a.length, b.length);
	for (let i = 0; i < shared; i++) {
		const diff = comparePathSegments(a[i], b[i]);
		if (diff !== 0) {
			return diff;
		}
	}
	return a.length - b.length;
}

function serializePathSegment(segment: PathSegment): string {
	if (typeof segment === "number") {
		return `n${segment}`;
	}
	return `l${segment.loop}:${segment.iteration}`;
}

function serializeLocation(location: Location): string {
	return location.map(serializePathSegment).join("|");
}

function buildSyntheticKey(
	location: Location,
	nameRegistry: readonly string[],
): string {
	return location
		.map((segment) => {
			if (typeof segment === "number") {
				return nameRegistry[segment] ?? `unknown-${segment}`;
			}
			return `~${segment.iteration}`;
		})
		.join("/");
}

function makeNode(
	id: string,
	x: number,
	y: number,
	data: WorkflowNodeInput,
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
			handledFailure: data.handledFailure,
			duration: data.duration,
			retryCount: data.retryCount,
			error: data.error,
			nodeKey: data.nodeKey,
			startedAt: data.startedAt,
			completedAt: data.completedAt,
			rawData: data.rawData,
			entryId: data.entryId,
			onReplayStep: data.onReplayStep,
		},
	};
}

function measureNode(node: AnyXYNode): { width: number; height: number } {
	const measuredWidth = node.measured?.width;
	const measuredHeight = node.measured?.height;
	const styleWidth =
		typeof node.style?.width === "number" ? node.style.width : undefined;
	const styleHeight =
		typeof node.style?.height === "number" ? node.style.height : undefined;

	return {
		width: measuredWidth ?? styleWidth ?? NODE_WIDTH,
		height: measuredHeight ?? styleHeight ?? NODE_HEIGHT,
	};
}

function getNodeBounds(node: AnyXYNode): Bounds {
	const { width, height } = measureNode(node);
	return {
		minX: node.position.x,
		minY: node.position.y,
		maxX: node.position.x + width,
		maxY: node.position.y + height,
	};
}

function mergeBounds(a: Bounds | null, b: Bounds | null): Bounds | null {
	if (!a) return b;
	if (!b) return a;
	return {
		minX: Math.min(a.minX, b.minX),
		minY: Math.min(a.minY, b.minY),
		maxX: Math.max(a.maxX, b.maxX),
		maxY: Math.max(a.maxY, b.maxY),
	};
}

function translateBounds(
	bounds: Bounds | null,
	dx: number,
	dy: number,
): Bounds | null {
	if (!bounds) {
		return null;
	}
	return {
		minX: bounds.minX + dx,
		minY: bounds.minY + dy,
		maxX: bounds.maxX + dx,
		maxY: bounds.maxY + dy,
	};
}

function translateFragment(
	fragment: LayoutFragment,
	dx: number,
	dy: number,
): LayoutFragment {
	return {
		...fragment,
		nodes: fragment.nodes.map((node) => ({
			...node,
			position: {
				x: node.position.x + dx,
				y: node.position.y + dy,
			},
		})),
		bounds: translateBounds(fragment.bounds, dx, dy),
	};
}

function gapLabel(
	completedAt: number | undefined,
	startedAt: number | undefined,
): Pick<Edge, "label" | "style" | "labelStyle" | "labelBgStyle"> | undefined {
	if (completedAt == null || startedAt == null || startedAt <= completedAt) {
		return undefined;
	}

	return {
		label: formatDuration(startedAt - completedAt),
		style: { stroke: "hsl(var(--muted-foreground))" },
		labelStyle: {
			fill: "hsl(var(--muted-foreground))",
			fontSize: 10,
		},
		labelBgStyle: {
			fill: "hsl(var(--background))",
			fillOpacity: 0.8,
		},
	};
}

function isTerminalFailureStatus(status: EntryStatus): boolean {
	return status === "failed" || status === "retrying";
}

function shouldMarkHandledFailure(
	node: RenderTreeEntryNode,
	context: LayoutContext,
): boolean {
	if (node.item.entry.kind.type !== "step") {
		return false;
	}

	return (
		isTerminalFailureStatus(deriveEntryStatus(node.item)) &&
		(context.hasTryAncestor || context.workflow.state !== "failed")
	);
}

function collectTreeStats(
	nodes: readonly RenderTreeNode[],
	workflow: WorkflowHistory,
	hasTryAncestor: boolean,
): TreeStats {
	const stats: TreeStats = {
		handledFailureNames: [],
		unhandledFailureNames: [],
		hasRunning: false,
		hasPending: false,
	};

	for (const node of nodes) {
		if (node.kind === "try") {
			const childStats = collectTreeStats(node.children, workflow, true);
			stats.handledFailureNames.push(...childStats.handledFailureNames);
			stats.unhandledFailureNames.push(
				...childStats.unhandledFailureNames,
			);
			stats.hasRunning ||= childStats.hasRunning;
			stats.hasPending ||= childStats.hasPending;
			continue;
		}

		const status = deriveEntryStatus(node.item);
		if (status === "running") {
			stats.hasRunning = true;
		}
		if (status === "pending") {
			stats.hasPending = true;
		}
		if (isTerminalFailureStatus(status)) {
			const isHandled = hasTryAncestor || workflow.state !== "failed";
			if (isHandled) {
				stats.handledFailureNames.push(node.name);
			} else {
				stats.unhandledFailureNames.push(node.name);
			}
		}

		const childStats = collectTreeStats(
			node.children,
			workflow,
			hasTryAncestor,
		);
		stats.handledFailureNames.push(...childStats.handledFailureNames);
		stats.unhandledFailureNames.push(...childStats.unhandledFailureNames);
		stats.hasRunning ||= childStats.hasRunning;
		stats.hasPending ||= childStats.hasPending;
	}

	return stats;
}

function summarizeTryGroup(
	node: RenderTreeTryNode,
	workflow: WorkflowHistory,
): { summary: string; handledFailureCount: number } {
	const stats = collectTreeStats(node.children, workflow, true);
	if (stats.handledFailureNames.length === 1) {
		return {
			summary: `caught ${stats.handledFailureNames[0]}`,
			handledFailureCount: 1,
		};
	}
	if (stats.handledFailureNames.length > 1) {
		return {
			summary: `caught ${stats.handledFailureNames.length} failures`,
			handledFailureCount: stats.handledFailureNames.length,
		};
	}
	if (stats.unhandledFailureNames.length > 0) {
		return {
			summary:
				stats.unhandledFailureNames.length === 1
					? `failed at ${stats.unhandledFailureNames[0]}`
					: `${stats.unhandledFailureNames.length} failures`,
			handledFailureCount: 0,
		};
	}
	if (stats.hasRunning) {
		return { summary: "running", handledFailureCount: 0 };
	}
	if (stats.hasPending) {
		return { summary: "pending", handledFailureCount: 0 };
	}
	return { summary: "completed", handledFailureCount: 0 };
}

function findBranchName(
	location: Location,
	containerLocation: Location,
	nameRegistry: readonly string[],
): string | null {
	const segment = location[containerLocation.length];
	if (typeof segment !== "number") {
		return null;
	}
	return nameRegistry[segment] ?? `unknown-${segment}`;
}

function buildRenderTree(workflow: WorkflowHistory): RenderTreeNode[] {
	const concreteNodes = new Map<string, RenderTreeEntryNode>();
	for (const item of workflow.history) {
		concreteNodes.set(serializeLocation(item.entry.location), {
			id: `entry:${item.entry.id}`,
			kind: "entry",
			item,
			key: item.key,
			name: getDisplayName(item.key),
			location: item.entry.location,
			children: [],
		});
	}

	const renderNodes = new Map<string, RenderTreeNode>(concreteNodes);

	for (const item of workflow.history) {
		for (let i = 1; i < item.entry.location.length; i++) {
			const prefix = item.entry.location.slice(0, i);
			const prefixKey = serializeLocation(prefix);
			if (renderNodes.has(prefixKey)) {
				continue;
			}

			const last = prefix[prefix.length - 1];
			if (typeof last !== "number") {
				continue;
			}

			const parentPrefix = prefix.slice(0, -1);
			const parentConcrete = concreteNodes.get(
				serializeLocation(parentPrefix),
			);
			if (
				parentConcrete &&
				(parentConcrete.item.entry.kind.type === "join" ||
					parentConcrete.item.entry.kind.type === "race")
			) {
				continue;
			}

			renderNodes.set(prefixKey, {
				id: `try:${prefixKey}`,
				kind: "try",
				key: buildSyntheticKey(prefix, workflow.nameRegistry),
				name: workflow.nameRegistry[last] ?? `unknown-${last}`,
				location: prefix,
				children: [],
			});
		}
	}

	const rootChildren: RenderTreeNode[] = [];
	const orderedNodes = Array.from(renderNodes.values()).sort((a, b) => {
		if (a.location.length !== b.location.length) {
			return a.location.length - b.location.length;
		}
		return compareLocations(a.location, b.location);
	});

	for (const node of orderedNodes) {
		let parentNode: RenderTreeNode | undefined;
		for (let i = node.location.length - 1; i > 0; i--) {
			parentNode = renderNodes.get(
				serializeLocation(node.location.slice(0, i)),
			);
			if (parentNode) {
				break;
			}
		}

		if (parentNode) {
			parentNode.children.push(node);
		} else {
			rootChildren.push(node);
		}
	}

	const sortTree = (nodes: RenderTreeNode[]) => {
		nodes.sort((a, b) => compareLocations(a.location, b.location));
		for (const node of nodes) {
			sortTree(node.children);
		}
	};
	sortTree(rootChildren);

	return rootChildren;
}

function layoutSequence(
	items: readonly RenderTreeNode[],
	context: LayoutContext,
): LayoutFragment {
	const nodes: AnyXYNode[] = [];
	const edges: Edge[] = [];
	let bounds: Bounds | null = null;
	let currentY = 0;
	let firstTargetId: string | undefined;
	let firstStartedAt: number | undefined;
	let prevNodeId: string | null = null;
	let prevCompletedAt: number | undefined;
	let pendingBranchSources: string[] = [];
	let outgoingSources: string[] = [];
	let lastCompletedAt: number | undefined;

	const connectTo = (targetId: string, targetStartedAt?: number) => {
		if (!firstTargetId) {
			firstTargetId = targetId;
			firstStartedAt = targetStartedAt;
		}
		if (prevNodeId) {
			edges.push({
				id: `e-${prevNodeId}-${targetId}`,
				source: prevNodeId,
				target: targetId,
				...gapLabel(prevCompletedAt, targetStartedAt),
			});
		}
		for (const source of pendingBranchSources) {
			edges.push({
				id: `e-${source}-${targetId}`,
				source,
				target: targetId,
			});
		}
		pendingBranchSources = [];
	};

	const applyContinuation = (
		nextSources: string[],
		completedAt: number | undefined,
	) => {
		if (nextSources.length === 1) {
			[prevNodeId] = nextSources;
			pendingBranchSources = [];
		} else if (nextSources.length > 1) {
			prevNodeId = null;
			pendingBranchSources = [...nextSources];
		} else {
			prevNodeId = null;
			pendingBranchSources = [];
		}
		outgoingSources = [...nextSources];
		prevCompletedAt = completedAt;
		lastCompletedAt = completedAt;
	};

	for (const item of items) {
		if (item.kind === "try") {
			const childFragment = layoutSequence(item.children, {
				...context,
				hasTryAncestor: true,
			});
			const childWidth = childFragment.bounds
				? childFragment.bounds.maxX - childFragment.bounds.minX
				: NODE_WIDTH;
			const groupWidth = Math.max(
				GROUP_WIDTH,
				childWidth + 2 * LOOP_PADDING_X,
			);
			const groupX = NODE_WIDTH / 2 - groupWidth / 2;
			const translatedChildren = translateFragment(
				childFragment,
				groupX + LOOP_PADDING_X - (childFragment.bounds?.minX ?? 0),
				currentY +
					LOOP_HEADER_HEIGHT -
					(childFragment.bounds?.minY ?? 0),
			);
			const groupHeight = translatedChildren.bounds
				? translatedChildren.bounds.maxY -
					currentY +
					LOOP_PADDING_BOTTOM
				: LOOP_HEADER_HEIGHT + LOOP_PADDING_BOTTOM;
			const summary = summarizeTryGroup(item, context.workflow);
			const groupNode: XYTryGroupNode = {
				id: item.id,
				type: "tryGroup",
				position: { x: groupX, y: currentY },
				measured: { width: groupWidth, height: groupHeight },
				style: { width: groupWidth, height: groupHeight },
				data: {
					label: item.name,
					summary: summary.summary,
					handledFailureCount: summary.handledFailureCount,
				},
			};

			connectTo(item.id, translatedChildren.firstStartedAt);
			nodes.push(groupNode, ...translatedChildren.nodes);
			edges.push(...translatedChildren.edges);
			bounds = mergeBounds(bounds, getNodeBounds(groupNode));
			bounds = mergeBounds(bounds, translatedChildren.bounds);

			currentY += groupHeight + NODE_GAP_Y;
			applyContinuation(
				translatedChildren.outgoingSources.length > 0
					? translatedChildren.outgoingSources
					: [item.id],
				translatedChildren.lastCompletedAt,
			);
			continue;
		}

		const entry = item.item.entry;
		const nodeData = itemToNodeData(item.item, context.workflow, {
			handledFailure: shouldMarkHandledFailure(item, context),
		});

		if (entry.kind.type === "loop") {
			const childFragment = layoutSequence(item.children, context);
			const childWidth = childFragment.bounds
				? childFragment.bounds.maxX - childFragment.bounds.minX
				: NODE_WIDTH;
			const groupWidth = Math.max(
				GROUP_WIDTH,
				childWidth + 2 * LOOP_PADDING_X,
			);
			const groupX = NODE_WIDTH / 2 - groupWidth / 2;
			const translatedChildren = translateFragment(
				childFragment,
				groupX + LOOP_PADDING_X - (childFragment.bounds?.minX ?? 0),
				currentY +
					LOOP_HEADER_HEIGHT -
					(childFragment.bounds?.minY ?? 0),
			);
			const groupHeight = translatedChildren.bounds
				? translatedChildren.bounds.maxY -
					currentY +
					LOOP_PADDING_BOTTOM
				: LOOP_HEADER_HEIGHT + LOOP_PADDING_BOTTOM;
			const groupId = `loop-${entry.id}`;
			const groupNode: XYLoopGroupNode = {
				id: groupId,
				type: "loopGroup",
				position: { x: groupX, y: currentY },
				measured: { width: groupWidth, height: groupHeight },
				style: { width: groupWidth, height: groupHeight },
				data: {
					label: item.name,
					summary: nodeData.summary,
				},
			};

			connectTo(groupId, nodeData.startedAt);
			nodes.push(groupNode, ...translatedChildren.nodes);
			edges.push(...translatedChildren.edges);
			bounds = mergeBounds(bounds, getNodeBounds(groupNode));
			bounds = mergeBounds(bounds, translatedChildren.bounds);

			currentY += groupHeight + NODE_GAP_Y;
			applyContinuation(
				translatedChildren.outgoingSources.length > 0
					? translatedChildren.outgoingSources
					: [groupId],
				nodeData.completedAt ?? translatedChildren.lastCompletedAt,
			);
			continue;
		}

		if (entry.kind.type === "join" || entry.kind.type === "race") {
			const headerId = `header-${entry.id}`;
			const headerNode = makeNode(headerId, 0, currentY, {
				...nodeData,
				label: item.name,
			});
			connectTo(headerId, nodeData.startedAt);
			nodes.push(headerNode);
			bounds = mergeBounds(bounds, getNodeBounds(headerNode));

			const branchData = entry.kind.data as JoinEntry | RaceEntry;
			const branchNames = Object.keys(branchData.branches);
			const branchStartY = currentY + NODE_HEIGHT + NODE_GAP_Y;
			const branchLayouts = branchNames.map((name) => {
				const branchItems = item.children.filter(
					(child) =>
						findBranchName(
							child.location,
							entry.location,
							context.workflow.nameRegistry,
						) === name,
				);
				const fragment = layoutSequence(branchItems, context);
				const fragmentWidth = fragment.bounds
					? fragment.bounds.maxX - fragment.bounds.minX
					: NODE_WIDTH;
				const requiredWidth = Math.max(
					GROUP_WIDTH,
					fragmentWidth + 2 * LOOP_PADDING_X,
				);
				const requiredHeight = fragment.bounds
					? fragment.bounds.maxY -
						fragment.bounds.minY +
						LOOP_HEADER_HEIGHT +
						LOOP_PADDING_BOTTOM
					: LOOP_HEADER_HEIGHT + LOOP_PADDING_BOTTOM;

				return {
					name,
					fragment,
					status: branchData.branches[name]?.status ?? "pending",
					isFailed:
						branchData.branches[name]?.status === "failed" ||
						branchData.branches[name]?.status === "cancelled",
					requiredWidth,
					requiredHeight,
				};
			});

			const branchWidth = Math.max(
				GROUP_WIDTH,
				...branchLayouts.map((layout) => layout.requiredWidth),
			);
			const branchHeight = Math.max(
				LOOP_HEADER_HEIGHT + LOOP_PADDING_BOTTOM,
				...branchLayouts.map((layout) => layout.requiredHeight),
			);
			const totalWidth =
				branchLayouts.length * branchWidth +
				Math.max(0, branchLayouts.length - 1) * BRANCH_GAP_X;
			const startX = NODE_WIDTH / 2 - totalWidth / 2;
			const nextSources: string[] = [];

			for (let i = 0; i < branchLayouts.length; i++) {
				const branch = branchLayouts[i];
				const branchX = startX + i * (branchWidth + BRANCH_GAP_X);
				const groupId = `branchgroup-${entry.id}-${branch.name}`;
				const translatedChildren = translateFragment(
					branch.fragment,
					branchX +
						LOOP_PADDING_X -
						(branch.fragment.bounds?.minX ?? 0),
					branchStartY +
						LOOP_HEADER_HEIGHT -
						(branch.fragment.bounds?.minY ?? 0),
				);
				const groupNode: XYBranchGroupNode = {
					id: groupId,
					type: "branchGroup",
					position: { x: branchX, y: branchStartY },
					measured: { width: branchWidth, height: branchHeight },
					style: { width: branchWidth, height: branchHeight },
					data: {
						label: branch.name,
						entryType: entry.kind.type,
						branchStatus: branch.status,
					},
				};

				nodes.push(groupNode, ...translatedChildren.nodes);
				edges.push({
					id: `e-${headerId}-${groupId}`,
					source: headerId,
					target: groupId,
				});
				edges.push(...translatedChildren.edges);
				bounds = mergeBounds(bounds, getNodeBounds(groupNode));
				bounds = mergeBounds(bounds, translatedChildren.bounds);

				if (!branch.isFailed) {
					nextSources.push(groupId);
				}
			}

			const hasTerminations = branchLayouts.some(
				(branch) => branch.isFailed,
			);
			const terminationY = branchStartY + branchHeight + TERMINATION_GAP;

			if (hasTerminations) {
				for (let i = 0; i < branchLayouts.length; i++) {
					const branch = branchLayouts[i];
					if (!branch.isFailed) {
						continue;
					}

					const branchX = startX + i * (branchWidth + BRANCH_GAP_X);
					const termId = `term-${entry.id}-${branch.name}`;
					const termNode: XYTerminationNode = {
						id: termId,
						type: "termination",
						position: {
							x:
								branchX +
								branchWidth / 2 -
								TERMINATION_NODE_SIZE / 2,
							y: terminationY,
						},
						measured: {
							width: TERMINATION_NODE_SIZE,
							height: TERMINATION_NODE_SIZE,
						},
						data: {},
					};

					nodes.push(termNode);
					edges.push({
						id: `e-branchgroup-${entry.id}-${branch.name}-${termId}`,
						source: `branchgroup-${entry.id}-${branch.name}`,
						target: termId,
					});
					bounds = mergeBounds(bounds, getNodeBounds(termNode));
				}
			}

			currentY =
				(hasTerminations
					? terminationY + TERMINATION_NODE_SIZE
					: branchStartY + branchHeight) + NODE_GAP_Y;
			applyContinuation(nextSources, nodeData.completedAt);
			continue;
		}

		const nodeId = `node-${entry.id}`;
		const node = makeNode(nodeId, 0, currentY, {
			...nodeData,
			label: item.name,
		});

		connectTo(nodeId, nodeData.startedAt);
		nodes.push(node);
		bounds = mergeBounds(bounds, getNodeBounds(node));
		currentY += NODE_HEIGHT + NODE_GAP_Y;
		applyContinuation([nodeId], nodeData.completedAt);
	}

	return {
		nodes,
		edges,
		bounds,
		firstTargetId,
		firstStartedAt,
		outgoingSources,
		lastCompletedAt,
	};
}

export function workflowHistoryToXYFlow(
	history: WorkflowHistory,
	options: WorkflowLayoutOptions = {},
): LayoutResult {
	const rootItems = buildRenderTree(history);
	const rootFragment = layoutSequence(rootItems, {
		workflow: history,
		hasTryAncestor: false,
	});

	const nodes: AnyXYNode[] = [];
	const edges: Edge[] = [];
	let currentY = 0;
	let prevNodeId: string | null = null;
	let prevCompletedAt: number | undefined;
	let pendingBranchSources: string[] = [];

	const connectTo = (targetId: string, targetStartedAt?: number) => {
		if (prevNodeId) {
			edges.push({
				id: `e-${prevNodeId}-${targetId}`,
				source: prevNodeId,
				target: targetId,
				...gapLabel(prevCompletedAt, targetStartedAt),
			});
		}
		for (const source of pendingBranchSources) {
			edges.push({
				id: `e-${source}-${targetId}`,
				source,
				target: targetId,
			});
		}
		pendingBranchSources = [];
	};

	const applyContinuation = (
		nextSources: string[],
		completedAt: number | undefined,
	) => {
		if (nextSources.length === 1) {
			[prevNodeId] = nextSources;
			pendingBranchSources = [];
		} else if (nextSources.length > 1) {
			prevNodeId = null;
			pendingBranchSources = [...nextSources];
		} else {
			prevNodeId = null;
			pendingBranchSources = [];
		}
		prevCompletedAt = completedAt;
	};

	if (history.input !== undefined) {
		const inputNode = makeNode("meta-input", 0, currentY, {
			label: "Input",
			summary: getEntrySummary("input", { value: history.input }),
			entryType: "input",
			status: "completed",
			nodeKey: "input",
			rawData: { value: history.input },
		});
		nodes.push(inputNode);
		currentY += NODE_HEIGHT + NODE_GAP_Y;
		applyContinuation(["meta-input"], undefined);
	}

	if (rootFragment.nodes.length > 0) {
		const translatedRoot = translateFragment(rootFragment, 0, currentY);
		if (translatedRoot.firstTargetId) {
			connectTo(
				translatedRoot.firstTargetId,
				translatedRoot.firstStartedAt,
			);
		}
		nodes.push(...translatedRoot.nodes);
		edges.push(...translatedRoot.edges);
		currentY += translatedRoot.bounds
			? translatedRoot.bounds.maxY -
				translatedRoot.bounds.minY +
				NODE_GAP_Y
			: 0;
		applyContinuation(
			translatedRoot.outgoingSources,
			translatedRoot.lastCompletedAt,
		);
	}

	if (history.output !== undefined && history.state === "completed") {
		const outputNode = makeNode("meta-output", 0, currentY, {
			label: "Output",
			summary: getEntrySummary("output", { value: history.output }),
			entryType: "output",
			status: "completed",
			nodeKey: "output",
			rawData: { value: history.output },
		});
		connectTo("meta-output");
		nodes.push(outputNode);
	}

	return { nodes, edges };
}
