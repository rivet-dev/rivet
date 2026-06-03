import { faCheck, Icon } from "@rivet-gg/icons";
import type { ReactNode } from "react";
import { cn } from "@/components";
import { Badge } from "@/components/ui/badge";

// Shared selection card for the agentOS onboarding steps. Mirrors the
// BuildTargetSelector card styling so the agentOS steps feel cohesive with the
// rest of the wizard. Single-select cards highlight; multi-select cards show a
// checkbox indicator.
export function SelectCard({
	title,
	description,
	badge,
	icon,
	selected,
	disabled,
	multi,
	onSelect,
}: {
	title: string;
	description: string;
	badge?: string;
	icon?: ReactNode;
	selected: boolean;
	disabled?: boolean;
	multi?: boolean;
	onSelect: () => void;
}) {
	return (
		<button
			type="button"
			disabled={disabled}
			aria-pressed={selected}
			onClick={disabled ? undefined : onSelect}
			className={cn(
				"flex items-start gap-3 rounded-lg border px-4 py-3 text-left transition-colors",
				disabled
					? "cursor-not-allowed opacity-50 border-border"
					: selected
						? "cursor-pointer border-primary bg-primary/5"
						: "cursor-pointer border-border hover:border-muted-foreground/50",
			)}
		>
			{multi ? (
				<span
					className={cn(
						"mt-0.5 flex size-4 shrink-0 items-center justify-center rounded border",
						selected
							? "border-primary bg-primary text-primary-foreground"
							: "border-muted-foreground/40",
					)}
				>
					{selected ? (
						<Icon icon={faCheck} className="size-2.5" />
					) : null}
				</span>
			) : icon != null ? (
				<span className="mt-0.5 shrink-0 text-muted-foreground">
					{icon}
				</span>
			) : null}
			<div className="min-w-0 flex-1">
				<div className="flex items-center gap-2">
					<p className="text-sm font-medium">{title}</p>
					{badge ? (
						<Badge
							variant="outline"
							className="text-[10px] leading-none py-0.5 px-1.5 font-medium"
						>
							{badge}
						</Badge>
					) : null}
				</div>
				<p className="text-xs text-muted-foreground">{description}</p>
			</div>
		</button>
	);
}
