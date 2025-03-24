import { MatchMakerLobbyConfigSettingsCard } from "@/domains/project/components/matchmaker-lobby-config-settings-card";
import { useEnvironment } from "@/domains/project/data/environment-context";
import { useProject } from "@/domains/project/data/project-context";
import * as Layout from "@/domains/project/layouts/matchmaker-layout";
import { Grid } from "@rivet-gg/components";
import { createFileRoute } from "@tanstack/react-router";

function MatchmakerSettingsView() {
	const { namespaceId: environmentId } = useEnvironment();
	const { gameId: projectId } = useProject();

	return (
		<Grid columns={{ initial: "1", md: "2" }} gap="4">
			<MatchMakerLobbyConfigSettingsCard
				projectId={projectId}
				environmentId={environmentId}
			/>
		</Grid>
	);
}

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/lobbies/settings",
)({
	staticData: {
		layout: "full",
	},
	component: MatchmakerSettingsView,
	pendingComponent: Layout.Content.Skeleton,
});
