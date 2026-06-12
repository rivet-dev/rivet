import { createFileRoute, notFound, redirect } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { OrgLanding, OrgLandingPending } from "@/app/org-landing";
import { RouteError } from "@/app/route-error";
import { RouteLayout } from "@/app/route-layout";

export const Route = createFileRoute("/_context/orgs/$organization/")({
	loader: async ({ context, params }) => {
		return match(context)
			.with({ __type: "cloud" }, async () => {
				const result = await context.queryClient.fetchInfiniteQuery(
					context.dataProvider.currentOrgProjectsQueryOptions(),
				);

				const hasProjects = (result.pages[0].projects?.length ?? 0) > 0;

				// New orgs go straight to onboarding. Orgs with projects land
				// on the org dashboard so users can pick a project (or jump
				// to members / billing) without using the breadcrumb.
				if (!hasProjects) {
					throw redirect({
						to: "/orgs/$organization/new",
						replace: true,
						search: true,
						params: {
							organization: params.organization,
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
	pendingComponent: OrgLandingPending,
	component: RouteComponent,
	errorComponent: RouteError,
});

function RouteComponent() {
	const { organization } = Route.useParams();
	return (
		<RouteLayout>
			<OrgLanding organization={organization} />
		</RouteLayout>
	);
}
