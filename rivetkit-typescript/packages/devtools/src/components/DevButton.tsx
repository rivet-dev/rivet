import {
	arrow,
	flip,
	offset,
	shift,
	useFloating,
	useHover,
	useInteractions,
} from "@floating-ui/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
	getCornerButtonStyle,
	getCornerFromPosition,
	useCornerPosition,
} from "../hooks/useCornerPosition";
import { useDraggable } from "../hooks/useDraggable";

const INDICATOR_PADDING = 20;
const STORAGE_KEY = "__rivetkit-devtools";

interface DevButtonProps {
	children: React.ReactNode;
	onClick?: () => void;
}

export function DevButton({ children, onClick }: DevButtonProps) {
	const [isOpen, setIsOpen] = useState(false);
	const arrowRef = useRef(null);

	const { refs, floatingStyles, context, middlewareData } = useFloating({
		open: isOpen,
		onOpenChange: setIsOpen,
		placement: "top",
		middleware: [
			offset(10),
			flip(),
			shift({ padding: 8 }),
			arrow({ element: arrowRef }),
		],
	});

	const hover = useHover(context, {
		delay: { open: 0, close: 0 },
	});

	const { getReferenceProps, getFloatingProps } = useInteractions([hover]);

	const { updateCorner, isRightSide, isBottom } = useCornerPosition({
		storageKey: STORAGE_KEY,
		defaultCorner: "bottom-right",
	});

	const handleBeforeSnap = useCallback((x: number, y: number) => {
		const newCorner = getCornerFromPosition(x, y);
		const isRightSide = newCorner.endsWith("right");
		const isBottom = newCorner.startsWith("bottom");

		return {
			...(isBottom
				? { bottom: INDICATOR_PADDING }
				: { top: INDICATOR_PADDING }),
			...(isRightSide
				? { right: INDICATOR_PADDING }
				: { left: INDICATOR_PADDING }),
		};
	}, []);

	const handleDragEnd = useCallback(
		(x: number, y: number) => {
			const newCorner = getCornerFromPosition(x, y);
			updateCorner(newCorner);
		},
		[updateCorner],
	);

	const {
		ref: dragRef,
		isDragging,
		hasDragged,
		handlers,
	} = useDraggable<HTMLButtonElement>({
		onBeforeSnap: handleBeforeSnap,
		onDragEnd: handleDragEnd,
	});

	const buttonStyle = getCornerButtonStyle({
		isBottom,
		isRightSide,
		isDragging,
		padding: INDICATOR_PADDING,
	});

	// Merge refs for draggable and floating UI
	const setRefs = useCallback(
		(node: HTMLButtonElement | null) => {
			dragRef.current = node;
			refs.setReference(node);
		},
		[refs],
	);

	// Close tooltip when dragging starts
	useEffect(() => {
		if (isDragging && isOpen) {
			setIsOpen(false);
		}
	}, [isDragging, isOpen]);

	const arrowX = middlewareData.arrow?.x;
	const arrowY = middlewareData.arrow?.y;

	// Get reference props but don't spread all of them to avoid conflicts
	const referenceProps = getReferenceProps();

	// Determine arrow side based on actual placement (after flip)
	const { placement } = context;
	const arrowSide = placement.split("-")[0];

	const arrowStyle = useMemo((): React.CSSProperties => {
		const style: React.CSSProperties = {
			left: arrowX != null ? `${arrowX}px` : "",
			top: arrowY != null ? `${arrowY}px` : "",
		};

		// Position arrow on the correct side
		if (arrowSide === "top") {
			style.bottom = "-4px";
		} else if (arrowSide === "bottom") {
			style.top = "-4px";
		} else if (arrowSide === "left") {
			style.right = "-4px";
		} else if (arrowSide === "right") {
			style.left = "-4px";
		}

		return style;
	}, [arrowX, arrowY, arrowSide]);

	const handleClick = useCallback(() => {
		if (!hasDragged && onClick) {
			onClick();
		}
	}, [hasDragged, onClick]);

	return (
		<>
			<button
				ref={setRefs}
				type="button"
				{...handlers}
				onClick={handleClick}
				onMouseEnter={
					referenceProps.onMouseEnter as React.MouseEventHandler<HTMLButtonElement>
				}
				onMouseLeave={
					referenceProps.onMouseLeave as React.MouseEventHandler<HTMLButtonElement>
				}
				style={buttonStyle}
			>
				{children}
			</button>
			{isOpen && !isDragging && (
				<div
					ref={refs.setFloating}
					className="tooltip-container"
					style={floatingStyles as React.CSSProperties}
					data-arrow-side={arrowSide}
					{...getFloatingProps()}
				>
					<div className="tooltip">Open Inspector</div>
					<div
						ref={arrowRef}
						className="tooltip-arrow"
						style={arrowStyle}
					/>
				</div>
			)}
		</>
	);
}
