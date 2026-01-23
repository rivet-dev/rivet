import { faChevronLeft, Icon } from "@rivet-gg/icons";
import { createFileRoute, Link } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { motion } from "framer-motion";
import z from "zod";
import CreateProjectFrameContent from "@/app/dialogs/create-project-frame";
import { SidebarlessHeader } from "@/app/layout";
import { Button, Card } from "@/components";
import { OnboardingFooter } from "@/components/onboarding/footer";
import { PathSelection } from "@/components/onboarding/path-selection";
import { TemplatesList } from "@/components/templates-list";
import { TEST_IDS } from "@/utils/test-ids";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/new/",
)({
	component: RouteComponent,
	validateSearch: zodValidator(
		z.object({
			flow: z.enum(["agent", "template", "manual"]).optional(),
			modal: z.string().optional(),
			noTemplate: z.coerce.boolean().optional(),
			showAll: z.coerce.boolean().optional(),
		}),
	),
});

function RouteComponent() {
	const search = Route.useSearch();
	const navigate = Route.useNavigate();
	const params = Route.useParams();

	if (!search.flow) {
		return (
			<>
				<SidebarlessHeader />
				<PathSelection />
			</>
		);
	}

	if (search.flow === "template") {
		return (
			<>
				<SidebarlessHeader />
				<TemplatesList
					back={
						<Link to="." search={{ flow: undefined }}>
							Back
						</Link>
					}
					getTemplateLink={(template) => ({
						to: "/orgs/$organization/new/$template",
						params: { template },
					})}
				/>
			</>
		);
	}

	return (
		<>
			<SidebarlessHeader />
			<div className="h-screen flex flex-col justify-safe-center">
				<div className="flex-1 flex flex-col justify-safe-center overflow-auto">
					<div className="flex mx-auto flex-1 w-full px-6 items-center justify-center">
						<div className="max-w-md w-full">
							<motion.div
								initial={{ opacity: 0, y: -20 }}
								animate={{ opacity: 1, y: 0 }}
								transition={{ duration: 0.3, delay: 0.5 }}
							>
								<Button
									className="mb-4 text-muted-foreground px-0.5 py-1 h-auto -mx-0.5"
									startIcon={<Icon icon={faChevronLeft} />}
									variant="link"
									size="xs"
									asChild
								>
									<Link to="." search={{ flow: undefined }}>
										Back
									</Link>
								</Button>
							</motion.div>

							<Card
								className="max-w-2xl w-full"
								data-testid={
									TEST_IDS.Onboarding.CreateProjectCard
								}
							>
								<CreateProjectFrameContent
									organization={params.organization}
									onSuccess={(data) => {
										return navigate({
											to: "/orgs/$organization/projects/$project",
											params: {
												organization:
													params.organization ?? "",
												project: data.project.name,
											},
											search: {
												noTemplate: true,
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
		</>
	);
}
