import { Icon, faExternalLink } from "@rivet-gg/icons";
import { BillingHeader } from "./billing-header";
import { BillingPlanPeriod } from "./billing-plan-period";
import { BillingPlans } from "./billing-plans";
import { BillingPortalButton } from "./billing-portal-button";

interface BillingProps {
	projectId: string;
	groupId: string;
}
export function Billing({ projectId, groupId }: BillingProps) {
	return (
		<>
			<BillingHeader
				projectId={projectId}
				lead={<BillingPlanPeriod />}
				actions={
					<>
						<BillingPortalButton
							groupId={groupId}
							intent="general"
							variant="secondary"
							endIcon={<Icon icon={faExternalLink} />}
						>
							Invoices
						</BillingPortalButton>
						<BillingPortalButton
							groupId={groupId}
							intent="payment_method_update"
							variant="secondary"
							endIcon={<Icon icon={faExternalLink} />}
						>
							Payment Method
						</BillingPortalButton>
					</>
				}
			/>
			<BillingPlans projectId={projectId} />
		</>
	);
}
