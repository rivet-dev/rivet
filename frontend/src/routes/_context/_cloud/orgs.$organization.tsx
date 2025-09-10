import { createFileRoute, Outlet } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { createOrganizationContext } from "@/app/data-providers/cloud-data-provider";

export const Route = createFileRoute("/_context/_cloud/orgs/$organization")({
	component: RouteComponent,
	context: ({ context, params }) => {
		return match(context)
			.with({ __type: "cloud" }, (context) => ({
				dataProvider: {
					...context.dataProvider,
					...createOrganizationContext({
						...context.dataProvider,
						organization: params.organization,
					}),
				},
			}))
			.otherwise(() => {
				throw new Error("Invalid context type for this route");
			});
	},
});

function RouteComponent() {
	return <Outlet />;
}
