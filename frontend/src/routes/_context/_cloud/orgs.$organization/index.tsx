import { createFileRoute, notFound, redirect } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { RouteError } from "@/app/route-error";
import { PendingRouteLayout } from "@/app/route-layout";
import { FullscreenLoading } from "@/components";

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
						search: true,
						params: {
							organization: params.organization,
							project: firstProject.name,
						},
					});
				}

				throw redirect({
					to: "/orgs/$organization/new",
					replace: true,
					search: true,
					params: {
						organization: params.organization,
					},
				});
			})
			.otherwise(() => {
				throw notFound();
			});
	},
	wrapInSuspense: true,
	pendingMinMs: 0,
	pendingMs: 0,
	pendingComponent: FullscreenLoading,
	component: RouteComponent,
	errorComponent: RouteError,
});

function RouteComponent() {
	return null;
}
