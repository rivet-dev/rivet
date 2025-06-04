import { Button, type ButtonProps } from "@rivet-gg/components";
import { portalBillingSessionQueryOptions } from "../../queries";
import { useSuspenseQuery } from "@tanstack/react-query";
import { useProject } from "../../data/project-context";

interface BillingPortalButtonProps extends ButtonProps {
	intent: "general" | "payment_method_update";
}

export function BillingPortalButton({
	intent,
	children,
	...props
}: BillingPortalButtonProps) {
	const {
		developer: { groupId },
	} = useProject();
	const { data } = useSuspenseQuery(
		portalBillingSessionQueryOptions({ groupId, intent }),
	);
	return (
		<Button type="button" {...props} asChild>
			<a href={data.stripeSessionUrl} target="_blank" rel="noreferrer">
				{children}
			</a>
		</Button>
	);
}
