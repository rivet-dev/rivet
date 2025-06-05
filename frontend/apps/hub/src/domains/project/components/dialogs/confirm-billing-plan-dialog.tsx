import type { DialogContentProps } from "@/hooks/use-dialog";
import type { Rivet } from "@rivet-gg/api-ee";
import {
	Button,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	Flex,
	Text,
} from "@rivet-gg/components";
import { useUpdateProjectBillingMutation } from "../../queries";
import { ProjectBillingPlanLabel } from "../billing/billing-plan-badge";

interface ContentProps extends DialogContentProps {
	plan: Rivet.ee.billing.Plan;
	projectId: string;
	onSuccess?: () => void;
}

export default function ConfirmBillingPlanDialogContent({
	plan,
	projectId,
	onSuccess,
	onClose,
}: ContentProps) {
	const { mutate, isPending } = useUpdateProjectBillingMutation({
		onSuccess: async () => {
			await onSuccess?.();
			onClose?.();
		},
	});

	return (
		<>
			<DialogHeader>
				<DialogTitle>Confirm Billing Plan Change</DialogTitle>
			</DialogHeader>
			<Flex gap="4" direction="col">
				<Text>
					Are you sure you want to change your current plan to{" "}
					<ProjectBillingPlanLabel plan={plan} />?
				</Text>
			</Flex>
			<DialogFooter>
				<Button variant="secondary" onClick={onClose}>
					Cancel
				</Button>
				<Button
					isLoading={isPending}
					onClick={() => {
						mutate({ plan, projectId });
					}}
				>
					Confirm
				</Button>
			</DialogFooter>
		</>
	);
}
