import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/_context/_cloud/new/")({
	component: RouteComponent,
	beforeLoad: async ({ context, params }) => {
		throw redirect({
			to: "/orgs/$organization/new",
			params: {
				organization: context.clerk.organization?.id ?? "",
				...params,
			},
		})
	},
});

function RouteComponent() {
	return null;
}
