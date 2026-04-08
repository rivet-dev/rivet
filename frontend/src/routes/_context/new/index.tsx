import { createFileRoute, redirect } from "@tanstack/react-router";
import { authClient } from "@/lib/auth";

export const Route = createFileRoute("/_context/new/")({
	component: RouteComponent,
	beforeLoad: async ({ params }) => {
		const session = await authClient.getSession();
		const orgId = session.data?.session?.activeOrganizationId ?? "";
		throw redirect({
			to: "/orgs/$organization/new",
			params: {
				organization: orgId,
				...params,
			},
			search: true,
		});
	},
});

function RouteComponent() {
	return null;
}
