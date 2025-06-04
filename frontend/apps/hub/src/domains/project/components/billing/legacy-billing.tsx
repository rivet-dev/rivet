import { Icon, faExternalLink } from "@rivet-gg/icons";
import { BillingHeader } from "./billing-header";
import { BillingPlanPeriod } from "./billing-plan-period";
import { BillingPlans } from "./billing-plans";
import { BillingPortalButton } from "./billing-portal-button";
import { useProject } from "../../data/project-context";

export function LegacyBilling() {
	const {
		gameId: projectId,
		developer: { groupId },
	} = useProject();

	return (
		<>
			<BillingHeader
				lead={<BillingPlanPeriod />}
				actions={
					<>
						<BillingPortalButton
							groupId={groupId}
							intent="general"
							variant="outline"
							endIcon={<Icon icon={faExternalLink} />}
						>
							Invoices
						</BillingPortalButton>
						<BillingPortalButton
							groupId={groupId}
							intent="payment_method_update"
							variant="outline"
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
