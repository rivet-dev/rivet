import { ActorsActorDetails } from "@rivet-gg/components/actors";
import type { ActorAtom } from "@rivet-gg/components/actors";
import { useEnvironment } from "../../data/environment-context";
import { useProject } from "../../data/project-context";
import { useExportActorLogsMutation } from "../../queries/actors/mutations";

interface ActorsActorDetailsWrapperProps {
	tab?: string;
	actor: ActorAtom;
	onTabChange?: (tab: string) => void;
}

export function ActorsActorDetailsWrapper({
	tab,
	actor,
	onTabChange,
}: ActorsActorDetailsWrapperProps) {
	const { nameId: projectNameId } = useProject();
	const { nameId: environmentNameId } = useEnvironment();
	const exportMutation = useExportActorLogsMutation();

	const handleExportLogs = async (
		actorId: string,
		_typeFilter?: string,
		_filter?: string,
	) => {
		// TODO: Add above filters
		const result = await exportMutation.mutateAsync({
			projectNameId,
			environmentNameId,
			queryJson: JSON.stringify({
				string_equal: {
					property: "actor_id",
					value: actorId,
				},
			}),
		});

		// Open the presigned URL in a new tab to download
		window.open(result.url, "_blank");
	};

	return (
		<ActorsActorDetails
			tab={tab}
			actor={actor}
			onTabChange={onTabChange}
			onExportLogs={handleExportLogs}
			isExportingLogs={exportMutation.isPending}
		/>
	);
}
