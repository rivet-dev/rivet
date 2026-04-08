import { createFileRoute, notFound, Outlet } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { authClient } from "@/lib/auth";

export const Route = createFileRoute("/_context/orgs/$organization")({
	component: RouteComponent,
	beforeLoad: async ({ context, params }) => {
		return await match(context)
			.with({ __type: "cloud" }, async (context) => {
				const org = await authClient.organization.getFullOrganization({
					query: { organizationSlug: params.organization },
				});

				if (org.error) {
					throw notFound();
				}

				const session = await authClient.getSession();
				if (
					session.data?.session.activeOrganizationId !== org.data.id
				) {
					await authClient.organization.setActive({
						organizationSlug: params.organization,
					});
				}

				return {
					dataProvider: context.getOrCreateOrganizationContext(
						context.dataProvider,
						org.data.slug,
					),
					org: org.data,
				};
			})
			.otherwise(() => {
				throw new Error("Invalid context type for this route");
			});
	},
	pendingMinMs: 0,
	pendingMs: 0,
});

function RouteComponent() {
	return <Outlet />;
}
