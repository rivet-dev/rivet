import { Slot } from "@radix-ui/react-slot";
import { type MouseEventHandler, useCallback } from "react";
import * as styles from "./styles/safe-hover.module.css";

export function SafeHover({
	children,
	offset = 0,
}: {
	children: React.ReactNode;
	offset?: number;
}) {
	const onMouseEnter: MouseEventHandler = useCallback(
		(e) => {
			const el = e.currentTarget as HTMLElement;
			const parentRect = (
				el.parentNode as HTMLElement
			)?.getBoundingClientRect();
			el.style.setProperty(
				"--safe-y0",
				`${el.getBoundingClientRect().top - parentRect.top + offset}px`,
			);

			el.style.setProperty(
				"--safe-y1",
				`${el.getBoundingClientRect().bottom - parentRect.top + offset}px`,
			);
		},
		[offset],
	);

	const onMouseMove: MouseEventHandler = useCallback((e) => {
		const el = e.currentTarget as HTMLElement;
		el.style.setProperty("--safe-x", `${e.nativeEvent.offsetX}px`);
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
