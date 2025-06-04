import { queryClient, rivetEeClient } from "@/queries/global";
import type { Rivet as RivetEe } from "@rivet-gg/api-ee";
import { useMutation } from "@tanstack/react-query";
import { projectBillingQueryOptions } from "../billing/query-options";
import { projectAggregateBillingQueryOptions } from "../query-options";

export const useUpdateProjectBillingMutation = ({
	onSuccess,
}: {
	onSuccess?: () => void;
}) => {
	return useMutation({
		mutationFn: ({
			projectId,
			plan,
		}: {
			groupId: string;
			projectId: string;
		} & RivetEe.ee.cloud.games.billing.UpdatePlanRequest) =>
			rivetEeClient.ee.cloud.games.billing.updatePlan(projectId, {
				plan,
			}),
		onSuccess: async (data, values) => {
			await queryClient.invalidateQueries(
				projectBillingQueryOptions(values.projectId),
			);
			await queryClient.invalidateQueries({
				...projectAggregateBillingQueryOptions({
					projectId: values.projectId,
					projectNameId: values.projectId,
					groupId: values.groupId,
				}),
				refetchType: "all",
			});
			onSuccess?.();
		},
	});
};
