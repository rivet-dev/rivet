import { useQuery } from "@tanstack/react-query";
import { useDataProvider } from "../data-provider";
import type { ActorId } from "../queries";

export const useActorInspectorData = (actorId: ActorId) => {
	const metadata = useQuery(useDataProvider().metadataQueryOptions());
	const inspectorToken = useQuery(
		useDataProvider().actorInspectorTokenQueryOptions(actorId),
	);

	return {
		token: inspectorToken.data || "",
		metadata: metadata.data,
		isError: metadata.isError || inspectorToken.isError,
	};
};
