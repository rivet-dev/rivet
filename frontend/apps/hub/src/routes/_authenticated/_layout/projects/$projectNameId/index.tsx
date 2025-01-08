import * as Layout from "@/domains/project/layouts/project-layout";
import { ProjectEnvironmentsView } from "@/domains/project/views/project-environments";
import { createFileRoute } from "@tanstack/react-router";

function ProjectIdRoute() {
	const { project } = Route.useRouteContext();
	return (
		<ProjectEnvironmentsView
			projectId={project.gameId}
			projectNameId={project.nameId}
		/>
	);
}

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/",
)({
	component: ProjectIdRoute,
	pendingComponent: Layout.Root.Skeleton,
});
