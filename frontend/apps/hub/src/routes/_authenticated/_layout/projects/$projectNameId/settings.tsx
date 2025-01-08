import * as Layout from "@/domains/project/layouts/project-settings-layout";
import { Outlet, createFileRoute } from "@tanstack/react-router";

function ProjectIdSettingsView() {
	const { projectNameId } = Route.useParams();
	return (
		<Layout.Root projectNameId={projectNameId}>
			<Outlet />
		</Layout.Root>
	);
}

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/settings",
)({
	component: ProjectIdSettingsView,
});
