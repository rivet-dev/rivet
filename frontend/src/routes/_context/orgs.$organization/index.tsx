import { createFileRoute, notFound, redirect } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { OrgLanding, OrgLandingPending } from "@/app/org-landing";
import { RouteError } from "@/app/route-error";
import { RouteLayout } from "@/app/route-layout";
import { features } from "@/lib/features";

export const Route = createFileRoute("/_context/orgs/$organization/")({
	loader: async ({ context, params }) => {
		return match(context)
			.with({ __type: "cloud" }, async () => {
				const [projects, clusters] = await Promise.all([
					context.queryClient.fetchInfiniteQuery(
						context.dataProvider.currentOrgProjectsQueryOptions(),
					),
					features.byoc
						? context.queryClient.fetchInfiniteQuery(
								context.dataProvider.currentOrgClustersQueryOptions(),
							)
						: undefined,
				]);

				const hasContent =
					(projects.pages[0].projects?.length ?? 0) > 0 ||
					(clusters?.pages[0].clusters?.length ?? 0) > 0;

				// New orgs go straight to onboarding. Orgs with projects or
				// clusters land on the org dashboard so users can pick one (or
				// jump to members / billing) without using the breadcrumb.
				if (!hasContent) {
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
