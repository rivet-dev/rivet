import type { DialogContentProps } from "@/hooks/use-dialog";
import {
	Button,
	cn,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@rivet-gg/components";
import { BillingPlans } from "../billing/billing-plans";
import type { Rivet } from "@rivet-gg/api-ee";
import { useState } from "react";
import { useUpdateProjectBillingMutation } from "../../queries";
import { ProjectBillingPlanLabel } from "../billing/billing-plan-badge";
import { useProject } from "../../data/project-context";

interface ContentProps extends DialogContentProps {}

export default function ChangePlanDialogContent({ onClose }: ContentProps) {
	const {
		gameId: projectId,
		developer: { groupId },
	} = useProject();
	const [plan, setPlan] = useState<Rivet.ee.billing.Plan | null>(null);

	const { mutate, isPending } = useUpdateProjectBillingMutation({
		onSuccess: async () => {
			onClose?.();
		},
	});

	return (
		<div className={cn(plan && "min-w-[36rem]", "flex gap-4 flex-col")}>
			<DialogHeader>
				<DialogTitle className="text-2xl">
					Manage Subscription
				</DialogTitle>
			</DialogHeader>

			{plan ? (
				<>
					<p>
						Are you sure you want to change your current plan to{" "}
						<b>
							<ProjectBillingPlanLabel plan={plan} />
						</b>
						?
					</p>
					<DialogFooter>
						<Button
							isLoading={isPending}
							onClick={() => {
								mutate({ plan, projectId, groupId });
							}}
						>
							Confirm
						</Button>
						<Button
							variant="secondary"
							onClick={() => setPlan(null)}
						>
							Cancel
						</Button>
					</DialogFooter>
				</>
			) : (
				<BillingPlans onSubscribe={setPlan} />
			)}
		</div>
	);
}
