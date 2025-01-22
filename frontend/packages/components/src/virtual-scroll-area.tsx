"use client";
import {
	type Virtualizer,
	type VirtualizerOptions,
	useVirtualizer,
} from "@tanstack/react-virtual";
import {
	type ComponentPropsWithoutRef,
	type FunctionComponent,
	type RefObject,
	useImperativeHandle,
} from "react";
import { cn } from "./lib/utils";
import { ScrollArea, type ScrollAreaProps } from "./ui/scroll-area";

// biome-ignore lint/suspicious/noExplicitAny: we don't care about the type of the row
interface VirtualScrollAreaProps<TItem extends Record<string, any>>
	// optional
	extends Partial<
			Omit<
				VirtualizerOptions<HTMLDivElement, Element>,
				"getScrollElement" | "estimateSize" | "count"
			>
		>,
		// required
		Pick<
			VirtualizerOptions<HTMLDivElement, Element>,
			"estimateSize" | "count"
		>,
		Pick<ScrollAreaProps, "viewportProps"> {
	getRowData: (index: number) => TItem;
	className?: string;
	row: FunctionComponent<TItem>;
	virtualizerRef?: RefObject<Virtualizer<HTMLDivElement, Element>>;
	viewportRef?: RefObject<HTMLDivElement>;
	scrollerProps?: ComponentPropsWithoutRef<"div">;
}

// biome-ignore lint/suspicious/noExplicitAny: we don't care about the type of the row
export function VirtualScrollArea<TItem extends Record<string, any>>({
	className,
	row: Row,
	getRowData,
	viewportProps,
	virtualizerRef,
	viewportRef,
	scrollerProps,
	...rowVirtualizerOptions
}: VirtualScrollAreaProps<TItem>) {
	const rowVirtualizer = useVirtualizer({
		...rowVirtualizerOptions,
		getScrollElement: () => viewportRef?.current || null,
	});

	useImperativeHandle(virtualizerRef, () => rowVirtualizer, [rowVirtualizer]);

	return (
		<ScrollArea
			viewportRef={viewportRef}
			className={className}
			viewportProps={viewportProps}
		>
			<div
				{...scrollerProps}
				className={cn("relative w-full", scrollerProps?.className)}
				style={{
					height: `${rowVirtualizer.getTotalSize()}px`,
				}}
			>
				{rowVirtualizer.getVirtualItems().map((virtualItem) => (
					<Row
						key={virtualItem.key}
						data-index={virtualItem.index}
						className="absolute w-full inset-x-0"
						ref={rowVirtualizer.measureElement}
						style={{
							transform: `translateY(${virtualItem.start}px)`,
						}}
						{...getRowData(virtualItem.index)}
					/>
				))}
			</div>
		</ScrollArea>
	);
}
