import * as Layout from "@/domains/project/layouts/group-settings-layout";
import { Outlet, createFileRoute } from "@tanstack/react-router";

function GroupIdSettingsView() {
	const { groupId } = Route.useParams();
	return (
		<Layout.Root groupId={groupId}>
			<Outlet />
		</Layout.Root>
	);
}

export const Route = createFileRoute(
	"/_authenticated/_layout/teams/$groupId/settings",
)({
	component: GroupIdSettingsView,
	pendingComponent: Layout.Root.Skeleton,
});
