import { GroupImageSettingsCard } from "@/domains/group/components/group-image-settings-card";
import { GroupNameSettingsCard } from "@/domains/group/components/group-name-settings-card";
import { Flex } from "@rivet-gg/components";
import { createFileRoute } from "@tanstack/react-router";

function GroupIdSettingsView() {
	const { groupId } = Route.useParams();
	return (
		<Flex gap="4" direction="col">
			<GroupNameSettingsCard groupId={groupId} />
			<GroupImageSettingsCard groupId={groupId} />
		</Flex>
	);
}

export const Route = createFileRoute(
	"/_authenticated/_layout/teams/$groupId/settings/",
)({
	component: GroupIdSettingsView,
	pendingComponent: () => {
		return (
			<Flex gap="4" direction="col">
				<GroupNameSettingsCard.Skeleton />
				<GroupImageSettingsCard.Skeleton />
			</Flex>
		);
	},
});
