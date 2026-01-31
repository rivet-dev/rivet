import { faExclamationTriangle, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Button, cn } from "@/components";
import { useCloudProjectDataProvider } from "@/components/actors";
import { useHighestUsagePercent } from "./hooks";

export function BillingLimitAlert() {
	const dataProvider = useCloudProjectDataProvider();
	const { data: billingData } = useQuery({
		...dataProvider.currentProjectBillingDetailsQueryOptions(),
	});

	const usagePercent = useHighestUsagePercent();
	const plan = billingData?.billing.activePlan || "free";

	if (plan !== "free" || usagePercent < 80) {
		return null;
	}

	return (
		<div
			className={cn(
				"mx-0.5 mb-2 p-2 rounded-md border  text-xs",
				usagePercent < 100 && "border-warning/60 bg-warning/10",
				usagePercent >= 100 &&
					"border-destructive/60 bg-destructive/20",
			)}
		>
			<div className="flex items-start gap-2">
				<Icon
					icon={faExclamationTriangle}
					className={cn(
						"text-warning shrink-0 mt-0.5",
						usagePercent >= 100 && "text-destructive",
					)}
				/>
				<div className="flex-1 flex min-w-0 items-center justify-center">
					<div className="flex-1">
						<p className="text-foreground font-medium">
							{usagePercent >= 100
								? "Usage limit reached"
								: "Approaching usage limit"}
						</p>
						<p className="text-muted-foreground mt-0.5">
							{usagePercent >= 100
								? "Upgrade to continue using Actors."
								: `You have used ${usagePercent}% of your plan's free usage.`}
						</p>
					</div>
					<div>
						<Button
							size="sm"
							className="h-7 text-xs"
							variant="ghost"
							asChild
						>
							<Link
								from="/orgs/$organization/projects/$project/ns/$namespace"
								to="/orgs/$organization/projects/$project/ns/$namespace/billing"
							>
								Upgrade
							</Link>
						</Button>
					</div>
				</div>
			</div>
		</div>
	);
}
