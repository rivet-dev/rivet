"use client";

import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";

interface DynamicNavWrapperProps {
	children: ReactNode;
	className?: string;
}

/**
 * Client-side wrapper that dynamically calculates sticky positioning based on parent element position.
 * 
 * This component is needed when sticky elements need to position themselves relative to a parent
 * element's position in the document, rather than using a fixed offset from the top of the viewport.
 * 
 * How it works:
 * 1. Gets the parent element's position relative to the viewport using getBoundingClientRect()
 * 2. Adds the current scroll position to get the absolute position from the top of the document
 * 3. Updates the sticky top position dynamically on resize events
 * 4. Uses "use client" directive to ensure DOM access and event handlers work properly
 */
export function DynamicNavWrapper({ children, className }: DynamicNavWrapperProps) {
	const containerRef = useRef<HTMLDivElement>(null);
	const [topPosition, setTopPosition] = useState('0');

	useEffect(() => {
		const updateTopPosition = () => {
			if (containerRef.current?.parentElement) {
				const parentRect = containerRef.current.parentElement.getBoundingClientRect();
				const scrollTop = window.scrollY || document.documentElement.scrollTop;
				const topRelativeToDocument = parentRect.top + scrollTop;
				setTopPosition(`${topRelativeToDocument}px`);
			}
		};

		updateTopPosition();
		window.addEventListener('resize', updateTopPosition);

		return () => {
			window.removeEventListener('resize', updateTopPosition);
		};
	}, []);

	return (
		<div 
			ref={containerRef}
			className={className}
			style={{ top: topPosition }}
		>
			{children}
		</div>
	);
}
