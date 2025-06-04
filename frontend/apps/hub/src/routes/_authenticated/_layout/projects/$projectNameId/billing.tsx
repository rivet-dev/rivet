import { createFileRoute } from "@tanstack/react-router";

import { BillingView } from "@/domains/project/views/billing-view";
import { guardEnterprise } from "@/lib/guards";
import { useProject } from "@/domains/project/data/project-context";
import { z } from "zod";
import { zodValidator } from "@tanstack/zod-adapter";
import { useDialog } from "@/hooks/use-dialog";

function Modals() {
	const navigate = Route.useNavigate();
	const search = Route.useSearch();

	const ChangePlanDialog = useDialog.ChangePlan.Dialog;

	const { modal } = search;

	const handleOnOpenChange = (value: boolean) => {
		if (!value) {
			navigate({ search: { modal: undefined } });
		}
	};
	return (
		<>
			<ChangePlanDialog
				dialogProps={{
					open: modal === "manage-plan",
					onOpenChange: handleOnOpenChange,
				}}
			/>
		</>
	);
}

function ProjectBillingRoute() {
	const { gameId: projectId } = useProject();

	return (
		<>
			<BillingView projectId={projectId} />
			<Modals />
		</>
	);
}

const searchSchema = z.object({
	modal: z.string().or(z.literal("manage-plan")).optional(),
});

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/billing",
)({
	validateSearch: zodValidator(searchSchema),
	beforeLoad: async ({ context: { queryClient } }) => {
		await guardEnterprise({ queryClient });
	},
	component: ProjectBillingRoute,
});
