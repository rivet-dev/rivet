import * as Layout from "@/domains/project/layouts/backend-layout";
import { ProjectBackendEnvironmentOverview } from "@/domains/project/views/environment-overview";
import { createFileRoute } from "@tanstack/react-router";

function ProjectBackendEnvironmentIdOverviewRoute() {
	const {
		environment: { namespaceId: environmentId },
		project: { gameId: projectId },
	} = Route.useRouteContext();

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
