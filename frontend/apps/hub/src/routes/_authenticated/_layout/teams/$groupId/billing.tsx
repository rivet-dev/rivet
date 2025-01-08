import { Flex, Text } from "@rivet-gg/components";
import { createFileRoute } from "@tanstack/react-router";

function GroupIdBillingView() {
	return (
		<Flex direction="row" gap="4">
			<Text>Billing</Text>
		</Flex>
	);
}

export const Route = createFileRoute(
	"/_authenticated/_layout/teams/$groupId/billing",
)({
	component: GroupIdBillingView,
});
