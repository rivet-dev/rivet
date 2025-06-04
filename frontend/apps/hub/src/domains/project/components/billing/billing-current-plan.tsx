import { Rivet } from "@rivet-gg/api-ee";
import { useBilling } from "./billing-context";
import {
	BillingCommunityPlan,
	BillingCustomPlan,
	BillingProPlan,
	BillingTeamPlan,
} from "./billing-plans";

interface BillingCurrentPlanProps {
	className?: string;
}

export function BillingCurrentPlan(props: BillingCurrentPlanProps) {
	const { activePlan } = useBilling();

	return <BillingPlan plan={activePlan} {...props} />;
}

interface BillingPlanProps extends BillingCurrentPlanProps {
	plan?: Rivet.ee.billing.Plan;
}

export function BillingPlan({ plan, ...props }: BillingPlanProps) {
	if (!plan) {
		return null;
	}

	if (plan === Rivet.ee.billing.Plan.Trial) {
		return <BillingCommunityPlan showPrice={false} {...props} />;
	}

	if (plan === Rivet.ee.billing.Plan.Indie) {
		return <BillingProPlan showPrice={false} {...props} />;
	}

	if (plan === Rivet.ee.billing.Plan.Studio) {
		return <BillingTeamPlan showPrice={false} {...props} />;
	}

	return <BillingCustomPlan showPrice={false} {...props} />;
}
