"use client";

import {
	Background,
	BackgroundVariant,
	Controls,
	MiniMap,
	type Node,
	type NodeMouseHandler,
	ReactFlow,
	ReactFlowProvider,
} from "@xyflow/react";
import { faRefresh, faXmark, Icon } from "@rivet-gg/icons";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import "@xyflow/react/dist/style.css";
import { cn, DiscreteCopyButton } from "@/components";
import { ActorObjectInspector } from "../console/actor-inspector";
import { workflowHistoryToXYFlow } from "./workflow-to-xyflow";
import type { WorkflowHistory } from "./workflow-types";
import {
	formatDuration,
	TYPE_COLORS,
	TypeIcon,
	type WorkflowNodeData,
	workflowNodeTypes,
} from "./xyflow-nodes";

type MetaExtendedEntryType =
	| "step"
	| "loop"
	| "sleep"
	| "message"
	| "rollback_checkpoint"
	| "join"
	| "race"
	| "removed"
	| "input"
	| "output";

function miniMapNodeColor(node: Node): string {
	const entryType = node.data?.entryType as string | undefined;
	if (typeof entryType === "string" && entryType in TYPE_COLORS) {
		return TYPE_COLORS[entryType as keyof typeof TYPE_COLORS].icon;
	}
	return "#3b82f6";
}

export function WorkflowVisualizer({
	workflow,
	currentStepId,
	rerunningEntryId,
	onRerunStep,
}: {
	workflow: WorkflowHistory;
	currentStepId?: string;
	rerunningEntryId?: string;
	onRerunStep?: (entryId: string) => void;
}) {
	const { nodes, edges } = useMemo(
		() =>
			workflowHistoryToXYFlow(workflow, {
				currentStepId,
				rerunningEntryId,
				onRerunStep,
			}),
		[workflow, currentStepId, rerunningEntryId, onRerunStep],
	);

	const [selectedNode, setSelectedNode] = useState<WorkflowNodeData | null>(
		null,
	);
	const [contextMenu, setContextMenu] = useState<{
		x: number;
		y: number;
		entryId: string;
		isRerunning: boolean;
		onRerunStep: (entryId: string) => void;
	} | null>(null);
	const contextMenuRef = useRef<HTMLDivElement | null>(null);

	const onNodeClick: NodeMouseHandler = useCallback((_event, node) => {
		if (node.type === "workflow" && node.data) {
			setSelectedNode(node.data as WorkflowNodeData);
			setContextMenu(null);
		}
	}, []);

	const onPaneClick = useCallback(() => {
		setSelectedNode(null);
		setContextMenu(null);
	}, []);

	const onNodeContextMenu: NodeMouseHandler = useCallback((event, node) => {
		if (node.type !== "workflow" || !node.data) {
			setContextMenu(null);
			return;
		}

		const data = node.data as WorkflowNodeData;
		if (
			!data.canRerun ||
			!data.entryId ||
			typeof data.onRerunStep !== "function"
		) {
			setContextMenu(null);
			return;
		}

		event.preventDefault();
		event.stopPropagation();
		setSelectedNode(data);
		setContextMenu({
			x: event.clientX,
			y: event.clientY,
			entryId: data.entryId,
			isRerunning: Boolean(data.isRerunning),
			onRerunStep: data.onRerunStep,
		});
	}, []);

	useEffect(() => {
		if (!contextMenu) {
			return;
		}

		const onPointerDown = (event: PointerEvent) => {
			if (
				contextMenuRef.current &&
				event.target instanceof globalThis.Node &&
				contextMenuRef.current.contains(event.target)
			) {
				return;
			}
			setContextMenu(null);
		};

		const onKeyDown = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				setContextMenu(null);
			}
		};

		window.addEventListener("pointerdown", onPointerDown);
		window.addEventListener("keydown", onKeyDown);

		return () => {
			window.removeEventListener("pointerdown", onPointerDown);
			window.removeEventListener("keydown", onKeyDown);
		};
	}, [contextMenu]);

	return (
		<div className="flex h-full w-full flex-col bg-background">
			<div className="relative flex min-h-0 flex-1">
				<ReactFlowProvider>
					<ReactFlow
						nodes={nodes}
						edges={edges}
						nodeTypes={workflowNodeTypes}
						fitView
						panOnScroll
						panOnDrag
						edgesFocusable={false}
						panActivationKeyCode={null}
						onNodeClick={onNodeClick}
						onNodeContextMenu={onNodeContextMenu}
						onPaneClick={onPaneClick}
						nodesDraggable={false}
						nodesConnectable={false}
						edgesReconnectable={false}
						proOptions={{ hideAttribution: true }}
					>
						<Background
							variant={BackgroundVariant.Dots}
							gap={20}
							size={1.5}
							color="hsl(var(--border))"
						/>
						<Controls />
						<MiniMap
							nodeColor={miniMapNodeColor}
							nodeStrokeColor="transparent"
							maskColor="hsl(20 14.3% 4.1% / 0.7)"
						/>
					</ReactFlow>
					{contextMenu && (
						<div
							ref={contextMenuRef}
							className="fixed z-50 min-w-[12rem] overflow-hidden rounded-md border bg-popover p-1 text-popover-foreground shadow-md"
							style={{ left: contextMenu.x, top: contextMenu.y }}
							onContextMenu={(event) => event.preventDefault()}
						>
							<button
								type="button"
								disabled={contextMenu.isRerunning}
								className={cn(
									"relative flex w-full select-none items-center gap-2 rounded-sm px-2 py-1.5 text-left text-sm outline-none transition-colors",
									contextMenu.isRerunning
										? "cursor-not-allowed opacity-50"
										: "cursor-default hover:bg-accent hover:text-accent-foreground",
								)}
								onClick={() => {
									contextMenu.onRerunStep(contextMenu.entryId);
									setContextMenu(null);
								}}
							>
								<Icon icon={faRefresh} />
								{contextMenu.isRerunning
									? "Rerunning from step..."
									: "Rerun from this step"}
							</button>
						</div>
					)}
				</ReactFlowProvider>
			</div>

			{/* Bottom details panel */}
			{selectedNode && (
				<div className="border-t border-border bg-card px-6 py-4">
					{/* Top row: identity + close */}
					<div className="flex items-center justify-between mb-3">
						<div className="flex items-center gap-3">
							<div
								className="flex h-10 w-10 items-center justify-center rounded-lg"
								style={{
									backgroundColor:
										TYPE_COLORS[
											selectedNode.entryType as MetaExtendedEntryType
										]?.iconBg,
								}}
							>
								<TypeIcon
									type={
										selectedNode.entryType as MetaExtendedEntryType
									}
									size={18}
								/>
							</div>
							<div>
								<div className="font-medium text-foreground">
									{selectedNode.label}
								</div>
								<div className="text-xs text-muted-foreground">
									{selectedNode.entryType}
								</div>
							</div>
							<div
								className={cn(
									"ml-2 rounded px-2 py-0.5 font-mono-console text-xs",
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
							{selectedNode.retryCount != null &&
								selectedNode.retryCount > 0 && (
									<div className="rounded bg-amber-500/10 px-2 py-0.5 font-mono-console text-xs text-amber-500">
										{selectedNode.retryCount} retry(s)
									</div>
								)}
						</div>
						<button
							type="button"
							onClick={() => setSelectedNode(null)}
							className="flex h-8 w-8 items-center justify-center rounded hover:bg-secondary"
						>
							<Icon
								icon={faXmark}
								className="text-muted-foreground"
							/>
						</button>
					</div>

					{/* Bottom row: metadata grid */}
					<div className="grid grid-cols-[1fr_auto_auto_auto] gap-4">
						<div
							className={cn(
								"min-w-0",
								!selectedNode.startedAt &&
									!selectedNode.completedAt &&
									selectedNode.duration === undefined &&
									"col-span-4",
							)}
						>
							<div className="mb-1 text-xs font-medium uppercase text-muted-foreground">
								Key
							</div>
							<DiscreteCopyButton
								value={
									selectedNode.nodeKey ?? selectedNode.label
								}
								size="sm"
								className="w-full text-sm text-left justify-between -mx-2"
							>
								<span className="truncate font-mono-console">
									{selectedNode.nodeKey ?? selectedNode.label}
								</span>
							</DiscreteCopyButton>
						</div>

						{selectedNode.startedAt && (
							<div className="border-l pl-4">
								<div className="mb-1 text-xs font-medium uppercase text-muted-foreground">
									Started
								</div>
								<div className="whitespace-nowrap rounded py-1 font-mono text-xs text-foreground">
									{new Date(
										selectedNode.startedAt,
									).toLocaleString()}
								</div>
							</div>
						)}

						{selectedNode.completedAt && (
							<div className="border-l pl-4">
								<div className="mb-1 text-xs font-medium uppercase text-muted-foreground">
									Completed
								</div>
								<div className="whitespace-nowrap rounded py-1 font-mono text-xs text-foreground">
									{new Date(
										selectedNode.completedAt,
									).toLocaleString()}
								</div>
							</div>
						)}

						{selectedNode.duration !== undefined && (
							<div className="border-l pl-4">
								<div className="mb-1 text-xs font-medium uppercase text-muted-foreground">
									Duration
								</div>
								<div className="rounded py-1 font-mono text-xs text-foreground">
									{formatDuration(selectedNode.duration)}
								</div>
							</div>
						)}
					</div>

					{/* Data + Error row */}
					{(selectedNode.rawData !== undefined ||
						selectedNode.error !== undefined) && (
						<div className="mt-3 grid grid-cols-1 gap-4">
							{selectedNode.rawData !== undefined && (
								<div className="min-w-0">
									<div className="mb-1 text-xs font-medium uppercase text-muted-foreground">
										Data
									</div>
									<pre className="max-h-36 overflow-auto rounded px-2 py-1 font-mono text-xs text-foreground">
										<ActorObjectInspector
											data={selectedNode.rawData}
										/>
									</pre>
								</div>
							)}

							{selectedNode.error !== undefined && (
								<div className="min-w-0">
									<div className="mb-1 text-xs font-medium uppercase text-destructive">
										Error
									</div>
									<div className="rounded bg-red-500/10 px-2 py-1 font-mono text-xs text-destructive">
										<ActorObjectInspector
											data={selectedNode.error}
										/>
									</div>
								</div>
							)}
						</div>
					)}
				</div>
			)}
		</div>
	);
}
