import { Slot } from "@radix-ui/react-slot";
import { type MouseEventHandler, type ReactNode, useCallback } from "react";
import styles from "./styles/safe-hover.module.css";

export function SafeHover({
	children,
	offset = 0,
}: {
	children: ReactNode;
	offset?: number;
}) {
	const onMouseEnter: MouseEventHandler = useCallback(
		(e) => {
			const el = e.currentTarget as HTMLElement;
			const parentRect = (
				el.parentNode as HTMLElement
			)?.getBoundingClientRect();

			if (!parentRect) return;

			const { top, bottom } = el.getBoundingClientRect();
			el.style.setProperty(
				"--safe-y0",
				`${top - parentRect.top + offset}px`,
			);

			el.style.setProperty(
				"--safe-y1",
				`${bottom - parentRect.top + offset}px`,
			);
		},
		[offset],
	);

	const onMouseMove: MouseEventHandler = useCallback((e) => {
		const el = e.currentTarget as HTMLElement;
		el.style.setProperty("--safe-x", `${e.nativeEvent.offsetX + 5}px`);
	}, []);

	return (
		<Slot
			onMouseEnter={onMouseEnter}
			onMouseMove={onMouseMove}
			className={styles.safeHover}
		>
			{children}
		</Slot>
	);
}
