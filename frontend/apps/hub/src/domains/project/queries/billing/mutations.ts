import { queryClient, rivetEeClient } from "@/queries/global";
import type { Rivet as RivetEe } from "@rivet-gg/api-ee";
import { useMutation } from "@tanstack/react-query";
import { projectBillingQueryOptions } from "../billing/query-options";

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
			projectId: string;
		} & RivetEe.ee.cloud.games.billing.UpdatePlanRequest) =>
			rivetEeClient.ee.cloud.games.billing.updatePlan(projectId, {
				plan,
			}),
		onSuccess: async (data, values) => {
			await queryClient.invalidateQueries(
				projectBillingQueryOptions(values.projectId),
			);
			onSuccess?.();
		},
	});
};