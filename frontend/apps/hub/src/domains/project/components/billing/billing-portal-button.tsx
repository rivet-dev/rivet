import { Button, type ButtonProps } from "@rivet-gg/components";
import { portalBillingSessionQueryOptions } from "../../queries";
import { useQuery } from "@tanstack/react-query";

interface BillingPortalButtonProps extends ButtonProps {
	groupId: string;
	intent: "general" | "payment_method_update";
}

export function BillingPortalButton({
	groupId,
	intent,
	...props
}: BillingPortalButtonProps) {
	const { data, isLoading } = useQuery(
		portalBillingSessionQueryOptions(groupId, intent),
	);

	return (
		<Button type="button" {...props} isLoading={isLoading} asChild>
			<a
				href={data?.stripeSessionUrl}
				target="_blank"
				rel="noopener noreferrer"
			>
				{props.children}
			</a>
		</Button>
	);
}
