import { useProject } from "@/domains/project/data/project-context";
import * as Layout from "@/domains/project/layouts/project-layout";
import { ProjectEnvironmentsView } from "@/domains/project/views/project-environments";
import { createFileRoute } from "@tanstack/react-router";

function ProjectIdRoute() {
	const { gameId: projectId, nameId: projectNameId } = useProject();
	return (
		<ProjectEnvironmentsView
			projectId={projectId}
			projectNameId={projectNameId}
		/>
	);
}

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/",
)({
	component: ProjectIdRoute,
	pendingComponent: Layout.Root.Skeleton,
});
