import { createFileRoute, Outlet } from "@tanstack/react-router";
import { match } from "ts-pattern";

export const Route = createFileRoute("/_context/_cloud/orgs/$organization")({
	component: RouteComponent,
	beforeLoad: async ({ context, params }) => {
		return await match(context)
			.with({ __type: "cloud" }, async (context) => {
				context.clerk.setActive({
					organization: params.organization,
				});

				return {
					dataProvider: context.getOrCreateOrganizationContext(
						context.dataProvider,
						params.organization,
					),
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
