import {
	faArrowDown,
	faArrowUp,
	faBolt,
	faCircleCheck,
	faCircleExclamation,
	faCircleXmark,
	faClock,
	faCodeMerge,
	faEnvelope,
	faFlag,
	faPlay,
	faRefresh,
	faSpinnerThird,
	faTrash,
	Icon,
} from "@rivet-gg/icons";
import {
	Handle,
	type Node,
	type NodeProps,
	NodeToolbar,
	Position,
} from "@xyflow/react";
import { useState } from "react";
import { cn } from "@/components";
import type { EntryKindType, EntryStatus } from "./workflow-types";

// Extended type for meta nodes
type MetaExtendedEntryType = EntryKindType | "input" | "output";

// Type colors - matching the SVG workflow visualizer design
export const TYPE_COLORS: Record<
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

export function TypeIcon({
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
			return (
				<Icon icon={faCodeMerge} style={{ color, fontSize: size }} />
			);
		case "race":
			return <Icon icon={faBolt} style={{ color, fontSize: size }} />;
		case "removed":
			return <Icon icon={faTrash} style={{ color, fontSize: size }} />;
		case "input":
			return (
				<Icon icon={faArrowDown} style={{ color, fontSize: size }} />
			);
		case "output":
			return <Icon icon={faArrowUp} style={{ color, fontSize: size }} />;
		default:
			return (
				<Icon icon={faCircleCheck} style={{ color, fontSize: size }} />
			);
	}
}

export function formatDuration(ms: number): string {
	if (ms < 1000) return `${ms}ms`;
	if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
	if (ms < 3600000) return `${(ms / 60000).toFixed(1)}m`;
	return `${(ms / 3600000).toFixed(1)}h`;
}

function statusColor(status: EntryStatus): string {
	switch (status) {
		case "completed":
			return "#22c55e";
		case "failed":
			return "#ef4444";
		case "running":
			return "#3b82f6";
		case "retrying":
			return "#f59e0b";
		default:
			return "#71717a";
	}
}

export interface WorkflowNodeData {
	[key: string]: unknown;
	label: string;
	summary: string;
	entryType: MetaExtendedEntryType;
	status: EntryStatus;
	duration?: number;
	retryCount?: number;
	error?: string;
	/** Full key path for display in the detail panel. */
	nodeKey?: string;
	/** Epoch ms when this entry started. */
	startedAt?: number;
	/** Epoch ms when this entry completed. */
	completedAt?: number;
	/** Raw entry data for the object inspector. */
	rawData?: unknown;
}

export type WorkflowNodeType = Node<WorkflowNodeData, "workflow">;

export const NODE_WIDTH = 200;
export const NODE_HEIGHT = 52;
export const LOOP_HEADER_HEIGHT = 40;
export const LOOP_PADDING_X = 16;
export const LOOP_PADDING_BOTTOM = 16;

export function WorkflowNode({ data, selected }: NodeProps<WorkflowNodeType>) {
	const colors = TYPE_COLORS[data.entryType];
	const isFailed = data.status === "failed";
	const [hovered, setHovered] = useState(false);

	return (
		<div
			className="relative"
			style={{ width: NODE_WIDTH, height: NODE_HEIGHT }}
			onPointerEnter={() => setHovered(true)}
			onPointerLeave={() => setHovered(false)}
		>
			{/* Card */}
			<div
				className={cn(
					"relative flex h-full w-full items-center rounded-[10px] border",
					"transition-all duration-150",
					isFailed && "border-2",
				)}
				style={{
					backgroundColor: colors.bg,
					borderColor: selected
						? "#52525b"
						: isFailed
							? "#ef4444"
							: colors.border,
				}}
			>
				<NodeToolbar
					isVisible={hovered}
					position={Position.Top}
					offset={8}
					className="rounded-lg border px-3 py-2 text-xs shadow-lg"
					style={{
						backgroundColor: "hsl(var(--popover))",
						borderColor: "hsl(var(--border))",
						color: "hsl(var(--popover-foreground))",
						maxWidth: 280,
					}}
				>
					<div className="flex flex-col gap-1">
						<div className="font-medium">{data.label}</div>
						<div
							className="flex items-center gap-1.5"
							style={{ color: "hsl(var(--muted-foreground))" }}
						>
							<TypeIcon type={data.entryType} size={10} />
							<span>{data.entryType}</span>
							<span>·</span>
							<span style={{ color: statusColor(data.status) }}>
								{data.status}
							</span>
						</div>
						{data.duration !== undefined && (
							<div
								style={{
									color: "hsl(var(--muted-foreground))",
								}}
							>
								Duration: {formatDuration(data.duration)}
							</div>
						)}
						{data.retryCount != null && data.retryCount > 0 && (
							<div style={{ color: "#f59e0b" }}>
								Retries: {data.retryCount}
							</div>
						)}
						{data.error && (
							<div style={{ color: "#ef4444" }}>
								Error: {data.error}
							</div>
						)}
					</div>
				</NodeToolbar>

				{/* Retry count badge */}
				{data.retryCount != null && data.retryCount > 0 && (
					<div
						className="absolute -top-2 right-1 flex items-center justify-center rounded px-1.5"
						style={{
							height: 16,
							background: "#18181b",
							border: `1px solid ${isFailed ? "#ef4444" : "#f59e0b"}`,
						}}
					>
						<span
							className="text-[9px] font-medium"
							style={{ color: isFailed ? "#ef4444" : "#f59e0b" }}
						>
							x{data.retryCount + 1}
						</span>
					</div>
				)}

				{/* Icon box */}
				<div
					className="ml-2.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-lg"
					style={{
						backgroundColor: colors.iconBg,
						border: `1px solid ${colors.icon}4d`,
					}}
				>
					<TypeIcon type={data.entryType} size={14} />
				</div>

				{/* Text */}
				<div className="ml-2 min-w-0 flex-1">
					<div
						className="truncate text-xs font-medium"
						style={{ color: "hsl(var(--foreground))" }}
					>
						{data.label.length > 18
							? `${data.label.slice(0, 18)}...`
							: data.label}
					</div>
					<div
						className="truncate text-[10px]"
						style={{ color: "hsl(var(--muted-foreground))" }}
					>
						{data.summary}
					</div>
				</div>

				{/* Duration */}
				{data.duration !== undefined &&
					!data.retryCount &&
					data.status !== "running" &&
					data.status !== "retrying" && (
						<span
							className="absolute bottom-1.5 right-3 text-[10px]"
							style={{ color: "#71717a" }}
						>
							{formatDuration(data.duration)}
						</span>
					)}

				{/* Running spinner */}
				{(data.status === "running" || data.status === "retrying") && (
					<div className="mr-2 flex h-6 w-6 shrink-0 items-center justify-center">
						<Icon
							icon={faSpinnerThird}
							className="animate-spin text-muted-foreground"
							style={{ fontSize: 20 }}
						/>
					</div>
				)}

				{/* Failed icon */}
				{isFailed && (
					<div className="mr-2 flex h-6 w-6 shrink-0 items-center justify-center">
						<Icon
							icon={faCircleExclamation}
							className="text-destructive"
							style={{ fontSize: 20 }}
						/>
					</div>
				)}
			</div>

			<Handle
				type="target"
				position={Position.Top}
				className="!bg-transparent !border-0 !w-0 !h-0"
			/>
			<Handle
				type="source"
				position={Position.Bottom}
				className="!bg-transparent !border-0 !w-0 !h-0"
			/>
		</div>
	);
}

export interface LoopGroupNodeData {
	[key: string]: unknown;
	label: string;
	summary: string;
}

export type LoopGroupNodeType = Node<LoopGroupNodeData, "loopGroup">;

export function LoopGroupNode({ data }: NodeProps<LoopGroupNodeType>) {
	const colors = TYPE_COLORS.loop;

	return (
		<div
			className="relative rounded-xl"
			style={{
				width: "100%",
				height: "100%",
				border: `1px dashed ${colors.icon}66`,
				backgroundColor: `${colors.icon}08`,
			}}
		>
			{/* Header */}
			<div
				className="flex items-center gap-2 px-3"
				style={{ height: LOOP_HEADER_HEIGHT }}
			>
				<div
					className="flex h-6 w-6 shrink-0 items-center justify-center rounded-md"
					style={{
						backgroundColor: colors.iconBg,
						border: `1px solid ${colors.icon}4d`,
					}}
				>
					<TypeIcon type="loop" size={11} />
				</div>
				<span
					className="text-xs font-medium"
					style={{ color: "hsl(var(--foreground))" }}
				>
					{data.label}
				</span>
				<span
					className="text-[10px]"
					style={{ color: "hsl(var(--muted-foreground))" }}
				>
					{data.summary}
				</span>
			</div>

			<Handle
				type="target"
				position={Position.Top}
				className="!bg-transparent !border-0 !w-0 !h-0"
			/>
			<Handle
				type="source"
				position={Position.Bottom}
				className="!bg-transparent !border-0 !w-0 !h-0"
			/>
		</div>
	);
}

export interface BranchGroupNodeData {
	[key: string]: unknown;
	label: string;
	entryType: "join" | "race";
	branchStatus: "pending" | "running" | "completed" | "failed" | "cancelled";
}

export type BranchGroupNodeType = Node<BranchGroupNodeData, "branchGroup">;

export function BranchGroupNode({ data }: NodeProps<BranchGroupNodeType>) {
	const colors = TYPE_COLORS[data.entryType];
	const isFailed =
		data.branchStatus === "failed" || data.branchStatus === "cancelled";
	const isCompleted = data.branchStatus === "completed";

	const accentColor = isFailed
		? "#ef4444"
		: isCompleted
			? "#22c55e"
			: colors.icon;
	const borderColor = `${accentColor}66`;
	const bgColor = `${accentColor}08`;

	return (
		<div
			className="relative rounded-xl"
			style={{
				width: "100%",
				height: "100%",
				border: `1px dashed ${borderColor}`,
				backgroundColor: bgColor,
			}}
		>
			{/* Header */}
			<div
				className="flex items-center gap-2 px-3"
				style={{ height: LOOP_HEADER_HEIGHT }}
			>
				<div
					className="flex h-6 w-6 shrink-0 items-center justify-center rounded-md"
					style={{
						backgroundColor: `${accentColor}15`,
						border: `1px solid ${accentColor}4d`,
					}}
				>
					<TypeIcon type={data.entryType} size={11} />
				</div>
				<span
					className="text-xs font-medium"
					style={{ color: "hsl(var(--foreground))" }}
				>
					{data.label}
				</span>
			</div>

			<Handle
				type="target"
				position={Position.Top}
				className="!bg-transparent !border-0 !w-0 !h-0"
			/>
			<Handle
				type="source"
				position={Position.Bottom}
				className="!bg-transparent !border-0 !w-0 !h-0"
			/>
		</div>
	);
}

// ─── Termination node (X) for failed/cancelled branches ──────

export const TERMINATION_NODE_SIZE = 28;

export interface TerminationNodeData {
	[key: string]: unknown;
}

export type TerminationNodeType = Node<TerminationNodeData, "termination">;

export function TerminationNode(_props: NodeProps<TerminationNodeType>) {
	return (
		<div
			className="flex items-center justify-center rounded-full"
			style={{
				width: TERMINATION_NODE_SIZE,
				height: TERMINATION_NODE_SIZE,
				backgroundColor: "hsl(var(--card))",
				border: "1px solid #ef444450",
			}}
		>
			<Icon
				icon={faCircleXmark}
				style={{ color: "#ef4444", fontSize: 14 }}
			/>
			<Handle
				type="target"
				position={Position.Top}
				className="!bg-transparent !border-0 !w-0 !h-0"
			/>
		</div>
	);
}

export const workflowNodeTypes = {
	workflow: WorkflowNode,
	loopGroup: LoopGroupNode,
	branchGroup: BranchGroupNode,
	termination: TerminationNode,
};
