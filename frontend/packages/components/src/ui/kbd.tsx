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
				"pointer-events-none  h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium opacity-100 inline-flex",
				className,
			)}
		>
			{children}
		</kbd>
	);
}

Kbd.Cmd = function Cmd() {
	return <span className="text-xs">⌘</span>;
};
