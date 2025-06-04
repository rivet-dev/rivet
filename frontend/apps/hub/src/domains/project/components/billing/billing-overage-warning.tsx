import { Alert, AlertTitle, Button, Flex } from "@rivet-gg/components";
import { Icon, faExclamationTriangle } from "@rivet-gg/icons";
import { Link } from "@tanstack/react-router";
import { BILLING_PLANS_CREDITS_VISIBILITY } from "../../data/billing-calculate-usage";
import { useOptionalBilling } from "./billing-context";
import { useProject } from "../../data/project-context";

export function BillingOverageWarning() {
	const billing = useOptionalBilling();
	const { nameId: projectNameId } = useProject();

	if (!billing) {
		return null;
	}

	const {
		activePlan,
		credits: { overage },
	} = billing;

	if (
		overage <= 0 ||
		!BILLING_PLANS_CREDITS_VISIBILITY.includes(activePlan)
	) {
		return null;
	}

	return (
		<Alert variant="destructive" className="animate-shake" mb="8">
			<Icon className="size-4" icon={faExclamationTriangle} />
			<Flex justify="between" items="center">
				<AlertTitle className="leading-normal">
					You have exceeded your credit limit for this billing period.
					<br />
					Please upgrade your plan to avoid service interruption.
				</AlertTitle>
				<Button size="sm" variant="destructive" asChild>
					<Link
						to="/projects/$projectNameId/billing"
						params={{ projectNameId }}
					>
						Manage Billing
					</Link>
				</Button>
			</Flex>
		</Alert>
	);
}
