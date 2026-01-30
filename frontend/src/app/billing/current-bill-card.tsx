"use client";

import { faInfoCircle, Icon } from "@rivet-gg/icons";
import { AnimatedCurrency, Button, formatCurrency } from "@/components";
import { WithTooltip } from "@/components/ui/tooltip";

interface CurrentBillTotalProps {
	total: number;
	periodStart: Date;
	periodEnd: Date;
}

export function CurrentBillTotal({
	total,
	periodStart,
	periodEnd,
}: CurrentBillTotalProps) {
	const daysRemaining = Math.ceil(
		(periodEnd.getTime() - Date.now()) / (1000 * 60 * 60 * 24),
	);

	return (
		<div className="rounded-lg border border-border bg-card p-6">
			<div className="flex items-center gap-1.5 text-sm text-muted-foreground">
				<span>Current bill total</span>
				<WithTooltip
					content="Total charges for the current billing period"
					delayDuration={0}
					trigger={
						<Button
							variant="ghost"
							size="icon"
							className="p-0 h-0 w-auto"
						>
							<Icon
								icon={faInfoCircle}
								className="h-3.5 w-3.5 cursor-help"
							/>
						</Button>
					}
				/>
			</div>
			<div className="mt-1 text-4xl font-semibold text-foreground">
				<AnimatedCurrency value={total} />
			</div>
			<div className="mt-2 text-sm text-muted-foreground">
				<span className="font-medium text-foreground">
					Billing period:
				</span>{" "}
				{periodStart.toLocaleDateString()} to{" "}
				{periodEnd.toLocaleDateString()}{" "}
				<span className="text-muted-foreground">
					({daysRemaining} days remaining)
				</span>
			</div>
		</div>
	);
}
