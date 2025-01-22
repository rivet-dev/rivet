import { Onboarding } from "@/components/onboarding/onboarding";
import { guardOssNewbie } from "@/lib/guards";
import { createFileRoute } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { z } from "zod";

function IndexRoute() {
	const navigate = Route.useNavigate();
	const { initialStep, project_name: projectName } = Route.useSearch();
	return (
		<Onboarding
			initialStep={initialStep}
			initialProjectName={projectName}
			onFinish={(project) => {
				return navigate({
					to: "/projects/$projectNameId/environments/$environmentNameId",
					params: {
						projectNameId: project.nameId,
						environmentNameId: "prod",
					},
				});
			}}
		/>
	);
}

const searchSchema = z.object({
	newbie: z.coerce.boolean().optional(),
	initialStep: z.coerce.number().optional(),
	project_name: z.coerce.string().optional(),
});

export const Route = createFileRoute("/_authenticated/_layout/")({
	validateSearch: zodValidator(searchSchema),
	component: IndexRoute,
	staticData: {
		layout: "onboarding",
	},
	beforeLoad: ({ search, context: { queryClient, auth } }) => {
		if (search.newbie === true) {
			return;
		}
		return guardOssNewbie({ queryClient, auth });
	},
	shouldReload: true,
});
