import { createFileRoute } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import z from "zod";
import CreateProjectFrameContent from "@/app/dialogs/create-project-frame";
import { SidebarlessHeader } from "@/app/layout";
import { Card } from "@/components";
import { OnboardingFooter } from "@/components/onboarding/footer";
import { TEST_IDS } from "@/utils/test-ids";

export const Route = createFileRoute(
	"/_context/orgs/$organization/new/",
)({
	component: RouteComponent,
	validateSearch: zodValidator(
		z.object({
			flow: z.enum(["agent", "manual"]).optional(),
			modal: z.string().optional(),
			showAll: z.coerce.boolean().optional(),
		}),
	),
});

function RouteComponent() {
	const search = Route.useSearch();
	const navigate = Route.useNavigate();
	const params = Route.useParams();

	return (
		<div className="h-screen flex flex-col overflow-hidden">
			<SidebarlessHeader />
			<div className="flex-1 min-h-0 flex flex-col overflow-hidden">
				<div className="flex-1 min-h-0 flex mx-auto w-full px-6 items-center justify-center overflow-auto">
					<div className="max-w-2xl w-full py-6">
						<Card
							className="max-w-2xl w-full"
							data-testid={TEST_IDS.Onboarding.CreateProjectCard}
						>
							<CreateProjectFrameContent
								organization={params.organization}
								onSuccess={(data, vars) => {
									return navigate({
										to: "/orgs/$organization/projects/$project",
										params: {
											organization: vars.organization,
											project: data.project.name,
										},
										search: {
											flow: search.flow,
										},
									});
								}}
							/>
						</Card>
					</div>
				</div>
				<OnboardingFooter />
			</div>
		</div>
	);
}
