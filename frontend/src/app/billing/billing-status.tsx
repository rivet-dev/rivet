import { useSuspenseQuery } from "@tanstack/react-query";
import { useCloudProjectDataProvider } from "@/components/actors";
import { PlanBadge } from "./billing-plan-badge";

export function BillingStatus() {
	const dataProvider = useCloudProjectDataProvider();
	const {
		data: { billing },
	} = useSuspenseQuery(
		dataProvider.currentProjectBillingDetailsQueryOptions(),
	);

	return (
		<p>
			You are currently on the{" "}
			<PlanBadge plan={billing?.activePlan ?? "free"} /> plan.{" "}
			{billing?.futurePlan &&
			billing.activePlan !== billing?.futurePlan &&
			billing.currentPeriodEnd ? (
				<>
					Your plan will change to{" "}
					<PlanBadge plan={billing.futurePlan} /> on{" "}
					{new Date(billing.currentPeriodEnd).toLocaleDateString(
						undefined,
						{
							year: "numeric",
							month: "long",
							day: "numeric",
						},
					)}
					.{" "}
				</>
			) : null}
		</p>
	);
}
