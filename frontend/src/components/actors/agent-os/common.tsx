// Shared helpers for the agentOS inspector tabs.

import { faCircle, Icon } from "@rivet-gg/icons";
import type { ReactNode } from "react";
import { cn } from "@/components";

/** Human-readable byte size, e.g. 4.6 MiB, 10 GiB. Returns "—" for nullish. */
export function formatBytes(bytes?: number): string {
	if (bytes == null) return "—";
	if (bytes < 1024) return `${bytes} B`;
	const units = ["KiB", "MiB", "GiB", "TiB"];
	let value = bytes / 1024;
	let unit = 0;
	while (value >= 1024 && unit < units.length - 1) {
		value /= 1024;
		unit += 1;
	}
	const formatted = value >= 100 ? value.toFixed(0) : value.toFixed(1);
	return `${formatted.replace(/\.0$/, "")} ${units[unit]}`;
}

/** Format an epoch-ms timestamp as "YYYY-MM-DD HH:mm UTC". */
export function formatUtc(ms?: number): string {
	if (ms == null) return "—";
	return `${new Date(ms).toISOString().slice(0, 16).replace("T", " ")} UTC`;
}

export type DotColor = "green" | "amber" | "red" | "muted";

const DOT_CLASS: Record<DotColor, string> = {
	green: "text-green-500",
	amber: "text-amber-500",
	red: "text-red-500",
	muted: "text-muted-foreground/50",
};

export function StatusDot({
	color,
	className,
}: {
	color: DotColor;
	className?: string;
}) {
	return (
		<Icon
			icon={faCircle}
			className={cn(
				"size-2 shrink-0 fill-current",
				DOT_CLASS[color],
				className,
			)}
		/>
	);
}

/** Centered empty/placeholder state filling the tab body. */
export function AgentOsEmpty({ children }: { children: ReactNode }) {
	return (
		<div className="flex h-full flex-1 items-center justify-center p-8 text-center text-sm text-muted-foreground">
			{children}
		</div>
	);
}

/** Sticky section header used at the top of several tabs. */
export function SectionHeader({
	title,
	description,
	actions,
}: {
	title: string;
	description?: string;
	actions?: ReactNode;
}) {
	return (
		<div className="flex items-start justify-between gap-3 border-b px-4 py-3">
			<div>
				<h3 className="text-sm font-semibold">{title}</h3>
				{description ? (
					<p className="mt-0.5 text-xs text-muted-foreground">
						{description}
					</p>
				) : null}
			</div>
			{actions}
		</div>
	);
}
