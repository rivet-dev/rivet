import type { SubmitHandler } from "@/domains/project/forms/cdn-new-custom-domain-form";
import { isRivetError } from "@/lib/utils";
import { queryClient } from "@/queries/global";
import { useCallback } from "react";
import {
	projectEnvironmentQueryOptions,
	projectQueryOptions,
	useEnvironmentAddDomainMutation,
} from "../queries";

interface UseCdnManageAuthUsersProps {
	projectId: string;
	environmentId: string;
	onSuccess?: () => void;
}

export function useCdnNewCustomDomainFormHandler({
	onSuccess,
	projectId,
	environmentId,
}: UseCdnManageAuthUsersProps) {
	const { mutateAsync } = useEnvironmentAddDomainMutation();

	return useCallback<SubmitHandler>(
		async (values, form) => {
			try {
				await mutateAsync({
					projectId,
					environmentId,
					domain: values.name,
				});
			} catch (error) {
				if (isRivetError(error)) {
					return form.setError("name", {
						type: "manual",
						message: error.body.message,
					});
				}
				return form.setError("name", {
					type: "manual",
					message: "Invalid domain name.",
				});
			}
			await queryClient.invalidateQueries(projectQueryOptions(projectId));
			await queryClient.invalidateQueries(
				projectEnvironmentQueryOptions({ projectId, environmentId }),
			);
			onSuccess?.();
		},
		[projectId, mutateAsync, environmentId, onSuccess],
	);
}
