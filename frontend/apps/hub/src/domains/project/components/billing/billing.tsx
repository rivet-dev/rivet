import { useSuspenseQuery } from "@tanstack/react-query";
import { useProject } from "../../data/project-context";
import { projectMetadataQueryOptions } from "../../queries";
import { ActorsBilling } from "./actors-billing";
import { LegacyBilling } from "./legacy-billing";

export function Billing() {
	const { gameId: projectId } = useProject();
	const {
		data: { legacyLobbiesEnabled },
	} = useSuspenseQuery(projectMetadataQueryOptions({ projectId }));

	if (legacyLobbiesEnabled) {
		return <LegacyBilling />;
	}

	return <ActorsBilling />;
}
