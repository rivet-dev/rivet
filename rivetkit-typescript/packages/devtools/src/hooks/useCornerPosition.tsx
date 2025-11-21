import { useEffect, useState } from "react";

type Corner = "top-left" | "top-right" | "bottom-left" | "bottom-right";

interface UseCornerPositionOptions {
	storageKey: string;
	defaultCorner?: Corner;
}

export function useCornerPosition(options: UseCornerPositionOptions) {
	const { storageKey, defaultCorner = "bottom-right" } = options;
	const [corner, setCorner] = useState<Corner>(defaultCorner);

	useEffect(() => {
		const saved = localStorage.getItem(storageKey);
		if (saved) {
			setCorner(saved as Corner);
		}
	}, [storageKey]);

	const updateCorner = (newCorner: Corner) => {
		setCorner(newCorner);
		localStorage.setItem(storageKey, newCorner);
	};

	const isRightSide = corner.endsWith("right");
	const isBottom = corner.startsWith("bottom");

	return {
		corner,
		updateCorner,
		isRightSide,
		isBottom,
	};
}

interface CornerButtonStyleOptions {
	isBottom: boolean;
	isRightSide: boolean;
	isDragging: boolean;
	padding?: number;
	paddingVertical?: number;
	paddingHorizontal?: number;
}

export function getCornerButtonStyle(
	options: CornerButtonStyleOptions,
): React.CSSProperties {
	const { isBottom, isRightSide, isDragging, padding = 20, paddingVertical, paddingHorizontal } = options;

	const verticalPadding = paddingVertical ?? padding;
	const horizontalPadding = paddingHorizontal ?? padding;

	return {
		position: "fixed",
		...(isBottom ? { bottom: verticalPadding } : { top: verticalPadding }),
		...(isRightSide ? { right: horizontalPadding } : { left: horizontalPadding }),
		transform: isDragging
			? "translate(var(--drag-x, 0px), var(--drag-y, 0px))"
			: undefined,
		cursor: isDragging ? "grabbing" : "pointer",
		flexDirection: isRightSide ? "row-reverse" : "row",
	};
}

export function getCornerFromPosition(x: number, y: number): Corner {
	const centerX = window.innerWidth / 2;
	const centerY = window.innerHeight / 2;

	if (x < centerX && y < centerY) {
		return "top-left";
	}
	if (x >= centerX && y < centerY) {
		return "top-right";
	}
	if (x < centerX && y >= centerY) {
		return "bottom-left";
	}
	return "bottom-right";
}
