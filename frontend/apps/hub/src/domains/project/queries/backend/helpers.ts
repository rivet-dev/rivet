import { useQuery } from "@tanstack/react-query";
import { useProjectBackendEnvDatabasePreviewMutation } from "./mutations";
import { projectBackendProjectEnvDatabasePreviewQueryOptions } from "./query-options";

export function useEnvironmentDatabasePreview(variables: {
	projectId: string;
	environmentId: string;
}) {
	const { isPending, mutateAsync } =
		useProjectBackendEnvDatabasePreviewMutation();
	const { data: cachedData } = useQuery(
		projectBackendProjectEnvDatabasePreviewQueryOptions(variables),
	);

	return {
		isLoading: isPending,
		data: cachedData,
		mutateAsync,
	};
}
