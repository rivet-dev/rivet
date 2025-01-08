import { bootstrapQueryOptions } from "@/domains/auth/queries/bootstrap";
import { isRivetError } from "@/lib/utils";
import { rivetEeClient } from "@/queries/global";
import type { QueryClient } from "@tanstack/react-query";
import { projectBackendQueryOptions } from "../queries";

export async function tryCreateBackend({
	projectId,
	environmentId,
	queryClient,
}: {
	projectId: string;
	environmentId: string;
	queryClient: QueryClient;
}) {
	const { cluster } = await queryClient.fetchQuery(bootstrapQueryOptions());

	if (cluster === "oss") {
		return;
	}

	try {
		await queryClient.fetchQuery(
			projectBackendQueryOptions({ projectId, environmentId }),
		);
	} catch (error) {
		if (isRivetError(error)) {
			if (error.body.code === "BACKEND_NOT_FOUND") {
				await rivetEeClient.ee.backend.create(
					projectId,
					environmentId,
					{},
				);
				await queryClient.invalidateQueries({
					...projectBackendQueryOptions({ projectId, environmentId }),
					refetchType: "all",
				});
				return;
			}
		}
	}
	return;
}
