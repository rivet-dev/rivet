import { GroupDetailedMembers } from "@/domains/group/views/group-detailed-members";
import * as Layout from "@/domains/project/layouts/group-layout";
import { Flex } from "@rivet-gg/components";
import { createFileRoute } from "@tanstack/react-router";

function GroupIdMembersView() {
	const { groupId } = Route.useParams();

	return (
		<Flex direction="row" gap="4">
			<GroupDetailedMembers groupId={groupId} />
		</Flex>
	);
}

export const Route = createFileRoute(
	"/_authenticated/_layout/teams/$groupId/members",
)({
	component: GroupIdMembersView,
	pendingComponent: Layout.Content.Skeleton,
});
