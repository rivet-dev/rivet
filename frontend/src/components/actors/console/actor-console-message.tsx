import {
	faAngleLeft,
	faAngleRight,
	faExclamationCircle,
	faSpinnerThird,
	faWarning,
	Icon,
} from "@rivet-gg/icons";
import { format } from "date-fns";
import { forwardRef, type ReactNode } from "react";
import { cn } from "@/components";

interface ActorConsoleMessageProps {
	variant:
		| "input"
		| "input-pending"
		| "output"
		| "error"
		| "log"
		| "warn"
		| "info"
		| "debug";
	timestamp?: Date;
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
				"whitespace-pre-wrap font-mono-console text-xs text-foreground/90 border-b pl-3 pr-5 flex py-1 -mt-[1px] gap-2",
				getConsoleMessageVariant(variant),
				className,
			)}
		>
			<div className="flex gap-1 w-4 items-center opacity-60 self-start min-h-4 flex-shrink-0 text-xs max-w-2">
				<ConsoleMessageVariantIcon variant={variant} />
			</div>
			<div className="min-h-4 text-foreground/30 flex-shrink-0 empty:hidden">
				{timestamp
					? format(timestamp, "LLL dd HH:mm:ss").toUpperCase()
					: null}
			</div>
			<div className="pl-2 min-h-4 flex-1 break-words min-w-0">
				{children}
			</div>
		</div>
	);
});

export const ConsoleMessageVariantIcon = ({
	variant,
	className,
}: {
	variant: string;
	className?: string;
}) => {
	if (variant === "input") {
		return <Icon icon={faAngleRight} className={cn("h-auto", className)} />;
	}
	if (variant === "input-pending") {
		return (
			<Icon
				icon={faSpinnerThird}
				className={cn("animate-spin h-auto", className)}
			/>
		);
	}
	if (variant === "output") {
		return <Icon icon={faAngleLeft} className={cn("h-auto", className)} />;
	}
	if (variant === "error") {
		return (
			<Icon
				icon={faExclamationCircle}
				className={cn("h-auto", className)}
			/>
		);
	}
	if (variant === "warn") {
		return <Icon icon={faWarning} className={cn("h-auto", className)} />;
	}
	return <span className="w-[11px]" />;
};

export const getConsoleMessageVariant = (variant: string) =>
	cn({
		"bg-red-950/30 border-red-800/40 text-red-400 z-10":
			variant === "error",
		"bg-yellow-500/10 border-yellow-800/40 text-yellow-200 z-10":
			variant === "warn",
		"bg-blue-950/30 border-blue-800/40 text-blue-400 z-10":
			variant === "debug",
	});
