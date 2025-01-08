import { Button, type ButtonProps } from "@rivet-gg/components";
import { useCreateBillingPortalSessionMutation } from "../../queries";

interface BillingPortalButtonProps extends ButtonProps {
	groupId: string;
	intent: "general" | "payment_method_update";
}

export function BillingPortalButton({
	groupId,
	intent,
	...props
}: BillingPortalButtonProps) {
	const { mutate, isPending } = useCreateBillingPortalSessionMutation();
	return (
		<Button
			type="button"
			{...props}
			isLoading={isPending}
			onClick={() => mutate({ groupId, intent })}
		/>
	);
}
