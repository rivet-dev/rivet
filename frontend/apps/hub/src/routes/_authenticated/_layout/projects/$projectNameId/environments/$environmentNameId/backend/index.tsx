import { useEnvironment } from "@/domains/project/data/environment-context";
import { useProject } from "@/domains/project/data/project-context";
import * as Layout from "@/domains/project/layouts/backend-layout";
import { ProjectBackendEnvironmentOverview } from "@/domains/project/views/environment-overview";
import { createFileRoute } from "@tanstack/react-router";

function ProjectBackendEnvironmentIdOverviewRoute() {
	const { namespaceId: environmentId } = useEnvironment();
	const { gameId: projectId } = useProject();

	return (
		<ProjectBackendEnvironmentOverview
			environmentId={environmentId}
			projectId={projectId}
		/>
	);
}

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/backend/",
)({
	component: ProjectBackendEnvironmentIdOverviewRoute,
	pendingComponent: Layout.Content.Skeleton,
});
