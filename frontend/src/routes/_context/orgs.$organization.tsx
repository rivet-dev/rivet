import { createFileRoute, Outlet, redirect } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { authClient } from "@/lib/auth";

export const Route = createFileRoute("/_context/orgs/$organization")({
	component: RouteComponent,
	context: ({ context, params }) =>
		match(context)
			.with({ __type: "cloud" }, (context) => ({
				dataProvider: context.getOrCreateOrganizationContext(
					context.dataProvider,
					params.organization,
				),
			}))
			.otherwise(() => {
				throw new Error("Invalid context type for this route");
			}),
	beforeLoad: async ({ params }) => {
		const org = await authClient.organization.getFullOrganization({
			query: { organizationSlug: params.organization },
		});

		// If the slug is unknown to the auth backend (stale URL, deleted org,
		// user not a member anymore), redirect to root rather than throwing
		// notFound(). notFound() leaves descendant matches stuck in `pending`
		// while their layout components keep rendering, which crashes
		// useCloudDataProvider() / useCloudProjectDataProvider() consumers.
		if (org.error) {
			throw redirect({ to: "/" });
		}

		const session = await authClient.getSession();
		if (session.data?.session.activeOrganizationId !== org.data.id) {
			await authClient.organization.setActive({
				organizationSlug: params.organization,
			});
		}

		return { org: org.data };
	},
	loader: ({ context }) => ({ dataProvider: context.dataProvider }),
	pendingMinMs: 0,
	pendingMs: 0,
});

function RouteComponent() {
	return <Outlet />;
}
