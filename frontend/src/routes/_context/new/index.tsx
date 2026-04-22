import { createFileRoute, redirect } from "@tanstack/react-router";
import { authClient } from "@/lib/auth";
import { features } from "@/lib/features";

export const Route = createFileRoute("/_context/new/")({
	component: RouteComponent,
	beforeLoad: async ({ params }) => {
		if (!features.auth) return;
		const session = await authClient.getSession();
		const activeOrgId = session.data?.session?.activeOrganizationId;
		if (!activeOrgId) return;
		const org = await authClient.organization.getFullOrganization({
			query: { organizationId: activeOrgId },
		});
		if (org.error || !org.data) return;
		throw redirect({
			to: "/orgs/$organization/new",
			params: {
				organization: org.data.slug,
				...params,
			},
			search: true,
		});
	},
});

function RouteComponent() {
	return null;
}
