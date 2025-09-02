"use client";
import type { ReactNode } from "react";
import { cn } from "../lib/utils";

interface KbdProps {
	className?: string;
	children: ReactNode;
}

export function Kbd({ className, children }: KbdProps) {
	return (
		<kbd
			className={cn(
				"pointer-events-none h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium opacity-100 inline-flex",
				className,
			)}
		>
			{children}
		</kbd>
	);
}

interface KbdKeyProps {
	className?: string;
}

Kbd.Key = function Key({ className }: KbdKeyProps) {
	return (
		<span className={cn("text-xs", className)} suppressHydrationWarning>
			{typeof window === "undefined"
				? " "
				: navigator?.userAgent?.includes("Mac")
					? "⌘"
					: "Ctrl"}
		</span>
	);
};
