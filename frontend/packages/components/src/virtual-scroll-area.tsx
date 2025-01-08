"use client";
import {
	type Virtualizer,
	type VirtualizerOptions,
	useVirtualizer,
} from "@tanstack/react-virtual";
import {
	type ReactElement,
	type RefObject,
	cloneElement,
	useImperativeHandle,
	useRef,
} from "react";
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
	row: ReactElement<TItem>;
	virtualizerRef?: RefObject<Virtualizer<HTMLDivElement, Element>>;
}

// biome-ignore lint/suspicious/noExplicitAny: we don't care about the type of the row
export function VirtualScrollArea<TItem extends Record<string, any>>({
	className,
	row: Row,
	getRowData,
	viewportProps,
	virtualizerRef,
	...rowVirtualizerOptions
}: VirtualScrollAreaProps<TItem>) {
	const ref = useRef<HTMLDivElement>(null);

	const rowVirtualizer = useVirtualizer({
		...rowVirtualizerOptions,
		getScrollElement: () => ref.current,
	});

	useImperativeHandle(virtualizerRef, () => rowVirtualizer, [rowVirtualizer]);

	return (
		<ScrollArea
			viewportRef={ref}
			className={className}
			viewportProps={viewportProps}
		>
			<div
				className="relative w-full"
				style={{
					height: `${rowVirtualizer.getTotalSize()}px`,
				}}
			>
				{rowVirtualizer.getVirtualItems().map((virtualItem) => (
					<div
						key={virtualItem.key}
						data-index={virtualItem.index}
						className="absolute w-full inset-x-0 px-4"
						ref={rowVirtualizer.measureElement}
						style={{
							transform: `translateY(${virtualItem.start}px)`,
						}}
					>
						{cloneElement(Row, {
							...getRowData(virtualItem.index),
						})}
					</div>
				))}
			</div>
		</ScrollArea>
	);
}
