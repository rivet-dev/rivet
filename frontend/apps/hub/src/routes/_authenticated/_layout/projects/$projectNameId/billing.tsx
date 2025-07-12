import { createFileRoute } from "@tanstack/react-router";

import { useProject } from "@/domains/project/data/project-context";
import { BillingView } from "@/domains/project/views/billing-view";
import { guardEnterprise } from "@/lib/guards";

function ProjectBillingRoute() {
	const { gameId: projectId } = useProject();

	return <BillingView projectId={projectId} />;
}

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/billing",
)({
	beforeLoad: async ({ context: { queryClient } }) => {
		await guardEnterprise({ queryClient });
	},
	component: ProjectBillingRoute,
});
