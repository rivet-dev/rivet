"use client";

import {
	faChevronRight,
	faCircle,
	faSpinnerThird,
	faWavePulse,
	Icon,
} from "@rivet-gg/icons";
import { useState } from "react";
import { cn } from "@/components";
import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from "@/components/ui/collapsible";
import type { SpanNode } from "./types";
import { formatDuration, isSpanInProgress } from "./utils";

interface SpanSidebarProps {
	spans: SpanNode[];
	selectedSpanId: string | null;
	selectedEventIndex: number | null;
	onSelectSpan: (spanId: string | null) => void;
	onSelectEvent: (spanId: string, eventIndex: number) => void;
}

export function SpanSidebar({
	spans,
	selectedSpanId,
	selectedEventIndex,
	onSelectSpan,
	onSelectEvent,
}: SpanSidebarProps) {
	return (
		<div className="w-72 border-r border-border bg-card flex flex-col">
			<div className="p-3 border-b border-border">
				<div className="flex items-center gap-2 text-sm font-medium text-foreground">
					<Icon icon={faWavePulse} />
					<span>Spans</span>
				</div>
			</div>
			<div className="flex-1 overflow-y-auto p-2">
				{spans.length === 0 ? (
					<div className="text-sm text-muted-foreground text-center py-4">
						No spans found
					</div>
				) : (
					<div className="space-y-1">
						{spans.map((span) => (
							<SpanTreeItem
								key={span.spanId}
								span={span}
								depth={0}
								selectedSpanId={selectedSpanId}
								selectedEventIndex={selectedEventIndex}
								onSelectSpan={onSelectSpan}
								onSelectEvent={onSelectEvent}
							/>
						))}
					</div>
				)}
			</div>
		</div>
	);
}

interface SpanTreeItemProps {
	span: SpanNode;
	depth: number;
	selectedSpanId: string | null;
	selectedEventIndex: number | null;
	onSelectSpan: (spanId: string | null) => void;
	onSelectEvent: (spanId: string, eventIndex: number) => void;
}

function SpanTreeItem({
	span,
	depth,
	selectedSpanId,
	selectedEventIndex,
	onSelectSpan,
	onSelectEvent,
}: SpanTreeItemProps) {
	const [isOpen, setIsOpen] = useState(true);
	const inProgress = isSpanInProgress(span);
	const hasChildren =
		span.children.length > 0 || (span.events && span.events.length > 0);
	const isSelected =
		selectedSpanId === span.spanId && selectedEventIndex === null;
	const durationMs = span.endNs
		? (span.endNs - span.startNs) / 1_000_000n
		: null;

	return (
		<Collapsible open={isOpen} onOpenChange={setIsOpen}>
			<div
				className="flex items-center gap-1"
				style={{ paddingLeft: `${depth * 12}px` }}
			>
				<CollapsibleTrigger
					className={cn(
						"size-5 flex items-center justify-center rounded hover:bg-accent transition-colors",
						!hasChildren && "opacity-0 pointer-events-none",
					)}
					disabled={!hasChildren}
				>
					<Icon
						icon={faChevronRight}
						className={cn(
							"size-3 text-muted-foreground transition-transform",
							isOpen && "rotate-90",
						)}
					/>
				</CollapsibleTrigger>

				<button
					onClick={() => onSelectSpan(span.spanId)}
					className={cn(
						"flex-1 flex items-center gap-2 px-2 py-1.5 rounded text-left text-sm transition-colors min-w-0",
						isSelected
							? "bg-accent text-accent-foreground"
							: "hover:bg-accent/50",
					)}
				>
					{inProgress && (
						<Icon
							icon={faSpinnerThird}
							className="size-3 shrink-0 animate-spin"
						/>
					)}
					<span className="truncate flex-1">{span.span.name}</span>
					{durationMs !== null && (
						<span className="text-xs text-muted-foreground shrink-0">
							{formatDuration(Number(durationMs))}
						</span>
					)}
				</button>
			</div>

			{hasChildren && (
				<CollapsibleContent>
					<div className="space-y-0.5">
						{/* Child spans */}
						{span.children.map((child) => (
							<SpanTreeItem
								key={child.spanId}
								span={child}
								depth={depth + 1}
								selectedSpanId={selectedSpanId}
								selectedEventIndex={selectedEventIndex}
								onSelectSpan={onSelectSpan}
								onSelectEvent={onSelectEvent}
							/>
						))}

						<ul
							style={{
								paddingLeft: `${(depth + 1) * 12 + 20}px`,
							}}
						>
							{/* Events */}
							{span.events?.map((event, idx) => {
								const isEventSelected =
									selectedSpanId === span.spanId &&
									selectedEventIndex === idx;

								return (
									<button
										key={`${span.spanId}-event-${idx}`}
										onClick={() =>
											onSelectEvent(span.spanId, idx)
										}
										className={cn(
											"w-full flex items-center gap-2 px-2 py-1.5 rounded text-left text-sm transition-colors",
											isEventSelected
												? "bg-accent text-accent-foreground"
												: "hover:bg-accent/50 text-muted-foreground",
										)}
									>
										<Icon
											icon={faCircle}
											className="size-2 shrink-0 fill-current"
										/>
										<span className="truncate">
											{event.name}
										</span>
									</button>
								);
							})}
						</ul>
					</div>
				</CollapsibleContent>
			)}
		</Collapsible>
	);
}
