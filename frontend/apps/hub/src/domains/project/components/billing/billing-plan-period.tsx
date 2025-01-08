import { format } from "date-fns";
import { useBilling } from "./billing-context";

export function BillingPlanPeriod() {
	const { subscription } = useBilling();

	if (!subscription) {
		return null;
	}

	return (
		<p>
			Billing period: {format(subscription.periodStartTs, "MMMM do")} -{" "}
			{format(subscription.periodEndTs, "MMMM do")}
		</p>
	);
}
