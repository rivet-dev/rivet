import { cn, WithTooltip } from "@/components";
import { useHighestUsagePercent } from "./hooks";

const progressColors = {
	low: {
		stroke: "stroke-emerald-500",
		background: "bg-emerald-950/50",
		border: "border-emerald-900/30",
	},
	medium: {
		stroke: "stroke-amber-500",
		background: "bg-amber-950/50",
		border: "border-amber-900/30",
	},
	high: {
		stroke: "stroke-orange-500",
		background: "bg-orange-950/50",
		border: "border-orange-900/30",
	},
};

export function BillingUsageGauge() {
	const progress = useHighestUsagePercent();

	if (progress < 50) {
		return null;
	}

	const radius = 9;
	const circumference = 2 * Math.PI * radius;
	const strokeDashoffset = circumference - (progress / 100) * circumference;

	const progressVariant =
		progress < 50 ? "low" : progress < 80 ? "medium" : "high";

	return (
		<WithTooltip
			content={`${progress}% of usage included in your plan`}
			trigger={
				<div
					className={cn(
						"rounded-full p-1 border",
						progressColors[progressVariant].border,
						progressColors[progressVariant].background,
					)}
				>
					<svg
						className="h-6 w-6"
						viewBox="0 0 24 24"
						fill="none"
						strokeWidth="3"
						strokeLinecap="round"
						aria-label="Usage Gauge"
						role="img"
					>
						<circle cx="12" cy="12" r={radius} />
						<circle
							cx="12"
							cy="12"
							r={radius}
							className={progressColors[progressVariant].stroke}
							strokeDasharray={circumference}
							strokeDashoffset={strokeDashoffset}
							transform="rotate(-90 12 12)"
						/>
					</svg>
				</div>
			}
		/>
	);
}
