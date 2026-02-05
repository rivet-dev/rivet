import { faQuestionCircle, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { createFileRoute, notFound, useNavigate } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { HelpDropdown } from "@/app/help-dropdown";
import { Content } from "@/app/layout";
import { RouteLayout } from "@/app/route-layout";
import { SidebarToggle } from "@/app/sidebar-toggle";
import { Button, H1, H3, H4 } from "@/components";
import { useCloudProjectDataProvider } from "@/components/actors";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/settings",
)({
	component: match(__APP_TYPE__)
		.with("cloud", () => RouteComponent)
		.otherwise(() => () => {
			throw notFound();
		}),
});

function RouteComponent() {
	return (
		<RouteLayout>
			<Content>
				<div>
					<div className="mb-4 pt-2 max-w-5xl mx-auto">
						<div className="flex justify-between items-center px-6 @6xl:px-0 py-4">
							<SidebarToggle className="absolute left-4" />
							<H1>Project Settings</H1>
							<HelpDropdown>
								<Button
									variant="outline"
									startIcon={<Icon icon={faQuestionCircle} />}
								>
									Need help?
								</Button>
							</HelpDropdown>
						</div>
					</div>

					<hr className="mb-6" />

					<div className="px-4">
						<DangerZone />
					</div>
				</div>
			</Content>
		</RouteLayout>
	);
}

function DangerZone() {
	const dataProvider = useCloudProjectDataProvider();
	const navigate = useNavigate();

	const { data: project } = useQuery(
		dataProvider.currentProjectQueryOptions(),
	);

	return (
		<div className="pb-4 pb-8 px-6 max-w-5xl mx-auto my-8 @6xl:border @6xl:rounded-lg">
			<div className="flex gap-2 items-center mb-2 mt-6">
				<H3>Danger Zone</H3>
			</div>
			<p className="mb-6 text-muted-foreground">
				Perform actions that could affect the stability of your project.
			</p>

			<div className="border border-destructive rounded-md p-4 bg-destructive/10 mb-4">
				<H4 className="mb-2 text-destructive-foreground">
					Archive project '{project?.displayName}'
				</H4>
				<p className=" mb-4">
					Archiving this project will permanently remove all associated
					namespaces, Rivet Actors, Runners, and configurations. This
					action cannot be undone.
				</p>
				<Button
					variant="destructive"
					onClick={() =>
						navigate({
							to: ".",
							search: {
								modal: "delete-project",
								displayName: project?.displayName,
							},
						})
					}
				>
					Archive Project
				</Button>
			</div>
		</div>
	);
}
