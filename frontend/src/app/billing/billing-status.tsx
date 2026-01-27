import { useSuspenseQuery } from "@tanstack/react-query";
import { useCloudProjectDataProvider } from "@/components/actors";
import { CurrentPlan } from "./billing-plans";

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
			<span className="font-semibold">
				<CurrentPlan plan={billing?.activePlan} />
			</span>{" "}
			plan.{" "}
			{billing?.futurePlan &&
			billing.activePlan !== billing?.futurePlan &&
			billing.currentPeriodEnd ? (
				<>
					Your plan will change to{" "}
					<span className="font-semibold">
						<CurrentPlan plan={billing.futurePlan} />
					</span>{" "}
					on{" "}
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
