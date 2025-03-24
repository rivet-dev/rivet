import { useEnvironment } from "@/domains/project/data/environment-context";
import { useProject } from "@/domains/project/data/project-context";
import * as Layout from "@/domains/project/layouts/project-layout";
import { EnvironmentVersions } from "@/domains/project/views/environment-versions";
import { createFileRoute } from "@tanstack/react-router";

function EnvironmentVersionsRoute() {
	const { gameId: projectId, nameId: projectNameId } = useProject();
	const { namespaceId: environmentId } = useEnvironment();
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
