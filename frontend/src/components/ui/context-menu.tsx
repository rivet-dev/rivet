"use client";

import * as React from "react";
import { createPortal } from "react-dom";
import { cn } from "../lib/utils";

interface ContextMenuState {
	open: boolean;
	x: number;
	y: number;
	setOpen: (open: boolean) => void;
	setPosition: (x: number, y: number) => void;
}

const ContextMenuStateContext = React.createContext<ContextMenuState | null>(
	null,
);

function useContextMenuState(): ContextMenuState {
	const value = React.useContext(ContextMenuStateContext);
	if (!value) {
		throw new Error("ContextMenu components must be used within ContextMenu");
	}
	return value;
}

function composeEventHandler<E>(
	original: ((event: E) => void) | undefined,
	next: (event: E) => void,
) {
	return (event: E) => {
		original?.(event);
		next(event);
	};
}

export function ContextMenu({ children }: { children: React.ReactNode }) {
	const [open, setOpen] = React.useState(false);
	const [position, setPositionState] = React.useState({ x: 0, y: 0 });

	const setPosition = React.useCallback((x: number, y: number) => {
		setPositionState({ x, y });
	}, []);

	return (
		<ContextMenuStateContext.Provider
			value={{
				open,
				x: position.x,
				y: position.y,
				setOpen,
				setPosition,
			}}
		>
			{children}
		</ContextMenuStateContext.Provider>
	);
}

export function ContextMenuTrigger({
	children,
	asChild,
}: {
	children: React.ReactElement<any>;
	asChild?: boolean;
}) {
	const { setOpen, setPosition } = useContextMenuState();

	const triggerProps = {
		onContextMenu: (event: React.MouseEvent) => {
			event.preventDefault();
			setPosition(event.clientX, event.clientY);
			setOpen(true);
		},
	};

	if (asChild) {
		return React.cloneElement(children, {
			...triggerProps,
			onContextMenu: composeEventHandler(
				children.props.onContextMenu,
				triggerProps.onContextMenu,
			),
		});
	}

	return <div {...triggerProps}>{children}</div>;
}

export function ContextMenuContent({
	children,
	className,
}: {
	children: React.ReactNode;
	className?: string;
}) {
	const { open, setOpen, x, y } = useContextMenuState();

	React.useEffect(() => {
		if (!open) {
			return;
		}

		const onPointerDown = () => setOpen(false);
		const onKeyDown = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				setOpen(false);
			}
		};

		window.addEventListener("pointerdown", onPointerDown);
		window.addEventListener("keydown", onKeyDown);

		return () => {
			window.removeEventListener("pointerdown", onPointerDown);
			window.removeEventListener("keydown", onKeyDown);
		};
	}, [open, setOpen]);

	if (!open || typeof document === "undefined") {
		return null;
	}

	return createPortal(
		<div
			className={cn(
				"fixed z-50 min-w-[10rem] overflow-hidden rounded-md border bg-popover p-1 text-popover-foreground shadow-md",
				className,
			)}
			style={{ left: x, top: y }}
			onContextMenu={(event) => event.preventDefault()}
		>
			{children}
		</div>,
		document.body,
	);
}

export function ContextMenuItem({
	children,
	className,
	disabled,
	onSelect,
}: {
	children: React.ReactNode;
	className?: string;
	disabled?: boolean;
	onSelect?: () => void;
}) {
	const { setOpen } = useContextMenuState();

	return (
		<button
			type="button"
			disabled={disabled}
			className={cn(
				"relative flex w-full select-none items-center rounded-sm px-2 py-1.5 text-left text-sm outline-none transition-colors",
				disabled
					? "cursor-not-allowed opacity-50"
					: "cursor-default hover:bg-accent hover:text-accent-foreground",
				className,
			)}
			onClick={() => {
				if (disabled) {
					return;
				}
				onSelect?.();
				setOpen(false);
			}}
		>
			{children}
		</button>
	);
}
