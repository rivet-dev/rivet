import { useEnvironment } from "@/domains/project/data/environment-context";
import { useProject } from "@/domains/project/data/project-context";
import * as Layout from "@/domains/project/layouts/backend-layout";
import { ProjectBackendEnvironmentVariables } from "@/domains/project/views/environment-variables";
import { createFileRoute } from "@tanstack/react-router";

function ProjectBackendEnvironmentIdVariablesRoute() {
	const { namespaceId: environmentId } = useEnvironment();
	const { gameId: projectId } = useProject();

	return (
		<ProjectBackendEnvironmentVariables
			projectId={projectId}
			environmentId={environmentId}
		/>
	);
}

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/backend/variables",
)({
	component: ProjectBackendEnvironmentIdVariablesRoute,
	pendingComponent: Layout.Content.Skeleton,
});
