import {
	Alert,
	AlertDescription,
	AlertTitle,
	Flex,
} from "@rivet-gg/components";
import { Icon, faCreditCard } from "@rivet-gg/icons";
import { BillingHeader } from "./billing-header";
import { BillingPortalButton } from "./billing-portal-button";

export function MissingPaymentMethod() {
	return (
		<div className="max-w-4xl w-full mx-auto px-4">
			<BillingHeader />
			<Alert>
				<Icon className="size-4" icon={faCreditCard} />
				<AlertTitle>Heads up!</AlertTitle>
				<AlertDescription>
					<Flex direction="col" items="start" gap="4">
						Add a payment method to your project.
						<BillingPortalButton intent="payment_method_update">
							Add payment method
						</BillingPortalButton>
					</Flex>
				</AlertDescription>
			</Alert>
		</div>
	);
}
