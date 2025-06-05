import {
	Alert,
	AlertDescription,
	AlertTitle,
	Flex,
} from "@rivet-gg/components";
import { Icon, faCreditCard } from "@rivet-gg/icons";
import { BillingHeader } from "./billing-header";
import { BillingPortalButton } from "./billing-portal-button";

interface MissingPaymentMethodProps {
	groupId: string;
	projectId: string;
}

export function MissingPaymentMethod({
	groupId,
	projectId,
}: MissingPaymentMethodProps) {
	return (
		<>
			<BillingHeader projectId={projectId} />
			<Alert>
				<Icon className="size-4" icon={faCreditCard} />
				<AlertTitle>Heads up!</AlertTitle>
				<AlertDescription>
					<Flex direction="col" items="start" gap="4">
						You must add a payment method before you can add servers
						to your project.
						<BillingPortalButton
							intent="payment_method_update"
							groupId={groupId}
						>
							Add payment method
						</BillingPortalButton>
					</Flex>
				</AlertDescription>
			</Alert>
		</>
	);
}
