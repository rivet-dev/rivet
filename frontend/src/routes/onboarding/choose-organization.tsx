import { createFileRoute, redirect } from "@tanstack/react-router";
import { Content } from "@/app/layout";
import { RouteLayout } from "@/app/route-layout";
import { authClient } from "@/lib/auth";

export const Route = createFileRoute("/onboarding/choose-organization")({
	component: RouteComponent,
	beforeLoad: async () => {
		const session = await authClient.getSession();
		if (!session.data) {
			throw redirect({ to: "/login" });
		}

		const orgs = await authClient.organization.list();

		if (orgs.data && orgs.data.length > 0) {
			await authClient.organization.setActive({
				organizationId: orgs.data[0].id,
			});
			throw redirect({
				to: "/orgs/$organization",
				params: { organization: orgs.data[0].id },
				search: true,
			});
		}

		// No orgs — auto-create a default org
		const user = session.data.user;
		const name = `${user.name || user.email.split("@")[0] || "Anonymous"}'s Organization`;
		const slug = `${name.toLowerCase().replace(/[^a-z0-9\s-]/g, "").replace(/\s+/g, "-").replace(/-+/g, "-").replace(/^-|-$/g, "")}-${Math.random().toString(36).substring(2, 6)}`;

		const newOrg = await authClient.organization.create({ name, slug });

		if (newOrg.data) {
			await authClient.organization.setActive({
				organizationId: newOrg.data.id,
			});
			throw redirect({
				to: "/orgs/$organization",
				params: { organization: newOrg.data.id },
				search: true,
			});
		}

		// Fallback — should not happen
		throw redirect({ to: "/login" });
	},
});

function RouteComponent() {
	return (
		<RouteLayout>
			<Content className="flex flex-col items-center justify-safe-center">
				<div className="w-full sm:w-96">
					Creating your organization...
				</div>
			</Content>
		</RouteLayout>
	);
}
