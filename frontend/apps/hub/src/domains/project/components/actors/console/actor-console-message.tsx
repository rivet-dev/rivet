import { cn } from "@rivet-gg/components";
import {
	Icon,
	faAngleLeft,
	faAngleRight,
	faSpinnerThird,
	faWarning,
	faXmark,
} from "@rivet-gg/icons";
import { format } from "date-fns";
import { type ReactNode, forwardRef } from "react";

interface ActorConsoleMessageProps {
	variant:
		| "input"
		| "input-pending"
		| "output"
		| "error"
		| "log"
		| "warn"
		| "debug";
	timestamp?: string;
	className?: string;
	children: ReactNode;
}

export const ActorConsoleMessage = forwardRef<
	HTMLDivElement,
	ActorConsoleMessageProps
>(({ children, variant, timestamp, className, ...props }, ref) => {
	return (
		<div
			ref={ref}
			{...props}
			className={cn(
				"whitespace-pre-wrap font-mono-console text-xs text-foreground/90 border-y pl-3 pr-5 flex py-1 -mt-[1px]",
				{
					"bg-red-950/30 border-red-800/40 text-red-400 z-10":
						variant === "error",
					"bg-yellow-500/10 border-yellow-800/40 text-yellow-200 z-10":
						variant === "warn",
					"bg-blue-950/30 border-blue-800/40 text-blue-400 z-10":
						variant === "debug",
				},
				className,
			)}
		>
			<div className="pl-2 flex gap-1 w-4 items-center opacity-60 self-start min-h-4 flex-shrink-0">
				{variant === "input" ? (
					<Icon icon={faAngleRight} className="max-w-1.5 h-auto" />
				) : null}
				{variant === "input-pending" ? (
					<Icon
						icon={faSpinnerThird}
						className="animate-spin max-w-4 h-auto"
					/>
				) : null}
				{variant === "output" ? (
					<Icon icon={faAngleLeft} className="max-w-1.5 h-auto" />
				) : null}
				{variant === "error" ? (
					<Icon icon={faXmark} className="max-w-4 h-auto" />
				) : null}
				{variant === "warn" ? (
					<Icon icon={faWarning} className="max-w-4 h-auto" />
				) : null}
			</div>
			<div className="pl-2 min-h-4 text-foreground/30 flex-shrink-0">
				{timestamp ? format(timestamp, "LLL dd HH:mm:ss O") : null}
			</div>
			<div className="pl-4 min-h-4 flex-1 break-words min-w-0">
				{children}
			</div>
		</div>
	);
});
