import { createFileRoute, notFound, redirect } from "@tanstack/react-router";
import { match } from "ts-pattern";
import CreateProjectFrameContent from "@/app/dialogs/create-project-frame";
import { Card } from "@/components";

export const Route = createFileRoute("/_context/_cloud/orgs/$organization/")({
	beforeLoad: ({ context, params }) => {
		return match(__APP_TYPE__)
			.with("cloud", async () => {
				if (!context.clerk?.organization) {
					throw notFound();
				}
				const result = await context.queryClient.fetchInfiniteQuery(
					context.dataProvider.currentOrgProjectsQueryOptions(),
				);

				const firstProject = result.pages[0].projects[0];

				if (firstProject) {
					throw redirect({
						to: "/orgs/$organization/projects/$project",
						replace: true,
						reloadDocument: true,
						params: {
							organization: params.organization,
							project: firstProject.name,
						},
					});
				}
			})
			.otherwise(() => {
				throw notFound();
			});
	},

	component: RouteComponent,
});

function RouteComponent() {
	return (
		<div className="flex flex-col gap-6 w-full mx-auto h-screen items-center justify-center overflow-auto">
			<Card className="w-full max-w-md">
				<CreateProjectFrameContent />
			</Card>
		</div>
	);
}
