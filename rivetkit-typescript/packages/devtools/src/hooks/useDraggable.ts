import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { flushSync } from "react-dom";

interface UseDraggableOptions {
	onDragEnd?: (x: number, y: number) => void;
	onBeforeSnap?: (
		x: number,
		y: number,
	) => { top?: number; bottom?: number; left?: number; right?: number };
}

export function useDraggable<T extends HTMLElement>(
	options?: UseDraggableOptions,
) {
	const [isDragging, setIsDragging] = useState(false);
	const [hasDragged, setHasDragged] = useState(false);
	const elementRef = useRef<T>(null);
	const dragStartRef = useRef({ x: 0, y: 0 });
	const animationRef = useRef<Animation | null>(null);
	const isPointerDownRef = useRef(false);

	// Cleanup animation on unmount
	useEffect(() => {
		return () => {
			if (animationRef.current) {
				animationRef.current.cancel();
				animationRef.current = null;
			}
		};
	}, []);

	const handlePointerDown = useCallback((e: React.PointerEvent) => {
		if (e.button !== 0) return;
		isPointerDownRef.current = true;
		setHasDragged(false);
		dragStartRef.current = { x: e.clientX, y: e.clientY };
		(e.target as HTMLElement).setPointerCapture(e.pointerId);
	}, []);

	const handlePointerMove = useCallback((e: React.PointerEvent) => {
		if (!elementRef.current || !isPointerDownRef.current) return;

		const offsetX = e.clientX - dragStartRef.current.x;
		const offsetY = e.clientY - dragStartRef.current.y;

		// Consider it a drag if moved more than 5 pixels
		if (Math.abs(offsetX) > 5 || Math.abs(offsetY) > 5) {
			setHasDragged(true);
			setIsDragging(true);
		}

		elementRef.current.style.setProperty("--drag-x", `${offsetX}px`);
		elementRef.current.style.setProperty("--drag-y", `${offsetY}px`);
	}, []);

	const handlePointerUpOrCancel = useCallback(
		(e: React.PointerEvent) => {
			if (!elementRef.current || !isPointerDownRef.current) return;
			isPointerDownRef.current = false;

			// Try to release pointer capture, but don't fail if it's already released
			try {
				(e.target as HTMLElement).releasePointerCapture(e.pointerId);
			} catch {
				// Ignore errors if pointer capture was already released
			}

			const element = elementRef.current;

			// Get current drag offset
			const dragOffsetX =
				Number.parseFloat(element.style.getPropertyValue("--drag-x")) ||
				0;
			const dragOffsetY =
				Number.parseFloat(element.style.getPropertyValue("--drag-y")) ||
				0;

			// If we didn't actually drag, just clean up and return
			if (Math.abs(dragOffsetX) <= 5 && Math.abs(dragOffsetY) <= 5) {
				element.style.setProperty("--drag-x", "0px");
				element.style.setProperty("--drag-y", "0px");
				setIsDragging(false);
				return;
			}

			// Get current position WITH drag offset applied
			const rect = element.getBoundingClientRect();

			// Calculate the final position based on current position + drag offset
			const finalX = rect.left + dragOffsetX + rect.width / 2;
			const finalY = rect.top + dragOffsetY + rect.height / 2;

			// Get the target corner position
			const targetPosition = options?.onBeforeSnap?.(finalX, finalY);

			// If no target position, just clean up
			if (!targetPosition) {
				element.style.setProperty("--drag-x", "0px");
				element.style.setProperty("--drag-y", "0px");
				setIsDragging(false);
				return;
			}

			// Calculate target screen position
			let targetScreenX = 0;
			let targetScreenY = 0;

			if (targetPosition.left !== undefined) {
				targetScreenX = targetPosition.left;
			} else if (targetPosition.right !== undefined) {
				targetScreenX =
					window.innerWidth - targetPosition.right - rect.width;
			}

			if (targetPosition.top !== undefined) {
				targetScreenY = targetPosition.top;
			} else if (targetPosition.bottom !== undefined) {
				targetScreenY =
					window.innerHeight - targetPosition.bottom - rect.height;
			}

			// Calculate the offset needed to reach target from current dragged position
			const deltaX = targetScreenX - rect.left;
			const deltaY = targetScreenY - rect.top;

			// Animate from current dragged position to target
			const animation = element.animate(
				[
					{
						transform: `translate(${dragOffsetX}px, ${dragOffsetY}px)`,
					},
					{
						transform: `translate(${dragOffsetX + deltaX}px, ${dragOffsetY + deltaY}px)`,
					},
				],
				{
					duration: 400,
					easing: "cubic-bezier(0.34, 1.2, 0.64, 1)",
					fill: "forwards",
				},
			);

			animationRef.current = animation;

			// Reset CSS variables
			element.style.setProperty("--drag-x", "0px");
			element.style.setProperty("--drag-y", "0px");

			// Wait for animation to finish before updating corner position
			animation.finished
				.then(() => {
					// Commit the animation effect by applying the final transform
					animation.commitStyles();
					// Cancel the animation to remove it from the element
					animation.cancel();
					animationRef.current = null;
					// Update state and corner position synchronously
					flushSync(() => {
						setIsDragging(false);
						options?.onDragEnd?.(finalX, finalY);
					});
					// Clean up the committed transform after React has rendered
					element.style.transform = "";
				})
				.catch(() => {
					// Animation was cancelled, clean up
					animationRef.current = null;
				});
		},
		[options],
	);

	const handlers = useMemo(
		() => ({
			onPointerDown: handlePointerDown,
			onPointerMove: handlePointerMove,
			onPointerUp: handlePointerUpOrCancel,
			onPointerCancel: handlePointerUpOrCancel,
		}),
		[handlePointerDown, handlePointerMove, handlePointerUpOrCancel],
	);

	return {
		ref: elementRef,
		isDragging,
		hasDragged,
		handlers,
	};
}
