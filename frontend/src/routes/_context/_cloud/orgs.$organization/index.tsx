import { createFileRoute, notFound, redirect } from "@tanstack/react-router";
import { match } from "ts-pattern";
import CreateProjectFrameContent from "@/app/dialogs/create-project-frame";
import { RouteError } from "@/app/route-error";
import { PendingRouteLayout, RouteLayout } from "@/app/route-layout";
import { Card, H2, Skeleton } from "@/components";

export const Route = createFileRoute("/_context/_cloud/orgs/$organization/")({
	loader: async ({ context, params }) => {
		return match(context)
			.with({ __type: "cloud" }, async () => {
				if (!context.clerk?.organization) {
					return;
				}
				const result = await context.queryClient.fetchInfiniteQuery(
					context.dataProvider.currentOrgProjectsQueryOptions(),
				);

				const firstProject = result.pages[0].projects[0];

				if (firstProject) {
					throw redirect({
						to: "/orgs/$organization/projects/$project",
						replace: true,

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
	wrapInSuspense: true,
	pendingMinMs: 0,
	pendingMs: 0,
	pendingComponent: PendingRouteLayout,
	component: RouteComponent,
	errorComponent: RouteError,
});

function RouteComponent() {
	return (
		<RouteLayout>
			<div className="bg-card h-full border my-2 mr-2 rounded-lg">
				<div className="mt-2 flex flex-col items-center justify-center h-full">
					<Card className="min-w-96">
						<CreateProjectFrameContent />
					</Card>
				</div>
			</div>
		</RouteLayout>
	);
}
