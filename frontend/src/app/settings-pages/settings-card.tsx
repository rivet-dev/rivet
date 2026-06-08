import type { ReactNode } from "react";
import { cn } from "@/components";

interface SettingsCardProps {
	/** Section heading rendered at the top of the card. */
	title?: ReactNode;
	/** Muted sub-heading rendered under the title. */
	description?: ReactNode;
	/** Right-aligned control in the header, e.g. an "Add" button. */
	action?: ReactNode;
	children?: ReactNode;
	/**
	 * Separate the body from the header with a divider instead of sharing the
	 * card's padding. Use for row lists (members, fields, danger items) where
	 * each row owns its own padding. Defaults to a single padded block, used
	 * for free-form content under a header.
	 */
	divided?: boolean;
	className?: string;
	/** Applied to the body wrapper. */
	contentClassName?: string;
}

/**
 * Canonical card chrome for the settings drawer. Every settings screen renders
 * its content in one of these so the border, radius, background, and inner
 * padding stay identical across screens. Pick `divided` for row lists and the
 * default padded mode for free-form content.
 */
export function SettingsCard({
	title,
	description,
	action,
	children,
	divided = false,
	className,
	contentClassName,
}: SettingsCardProps) {
	const hasHeader = Boolean(title || description || action);

	const header = hasHeader ? (
		<div className="flex items-start justify-between gap-3">
			<div className="min-w-0">
				{title ? (
					<h3 className="text-sm font-semibold text-foreground">
						{title}
					</h3>
				) : null}
				{description ? (
					<p className="mt-0.5 text-xs text-muted-foreground">
						{description}
					</p>
				) : null}
			</div>
			{action ? <div className="shrink-0">{action}</div> : null}
		</div>
	) : null;

	if (divided) {
		return (
			<div
				className={cn(
					"overflow-hidden rounded-lg border border-foreground/10 bg-card",
					className,
				)}
			>
				{header ? <div className="px-5 pt-5 pb-4">{header}</div> : null}
				{children ? (
					<div
						className={cn(
							hasHeader && "border-t border-foreground/10",
							contentClassName,
						)}
					>
						{children}
					</div>
				) : null}
			</div>
		);
	}

	return (
		<div
			className={cn(
				"rounded-lg border border-foreground/10 bg-card p-5",
				className,
			)}
		>
			{header ? <div className="mb-3">{header}</div> : null}
			{children ? (
				<div className={contentClassName}>{children}</div>
			) : null}
		</div>
	);
}
