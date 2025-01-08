import * as Layout from "@/domains/project/layouts/project-layout";
import { EnvironmentVersions } from "@/domains/project/views/environment-versions";
import { createFileRoute } from "@tanstack/react-router";

function EnvironmentVersionsRoute() {
	const {
		project: { gameId: projectId, nameId: projectNameId },
		environment: { namespaceId: environmentId },
	} = Route.useRouteContext();
	return (
		<EnvironmentVersions
			projectId={projectId}
			projectNameId={projectNameId}
			environmentId={environmentId}
		/>
	);
}

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/versions",
)({
	component: EnvironmentVersionsRoute,
	pendingComponent: Layout.Root.Skeleton,
});
