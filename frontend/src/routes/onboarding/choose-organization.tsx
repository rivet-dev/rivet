import { createFileRoute, redirect } from "@tanstack/react-router";
import { Content } from "@/app/layout";
import { RouteLayout } from "@/app/route-layout";

export const Route = createFileRoute("/onboarding/choose-organization")({
	component: RouteComponent,
	beforeLoad: async ({ context }) => {
		// Defensive check - session should be available after waitForClerk in parent route
		if (!context.clerk.session) {
			console.error("[choose-organization] No session available", {
				hasUser: !!context.clerk.user,
				status: context.clerk.status,
			});
			throw new Error("Session not available. Please try refreshing the page.");
		}

		if (context.clerk.organization) {
			throw redirect({
				to: "/orgs/$organization",
				params: { organization: context.clerk.organization.id },
				search: true,
			});
		}

		const org = await context.clerk.createOrganization({
			name: `${context.clerk.user?.firstName || context.clerk.user?.primaryEmailAddress?.emailAddress.split("@")[0] || "Anonymous"}'s Organization`,
		});

		await context.clerk.setActive({ organization: org.id });
		await context.clerk.session?.reload();

		throw redirect({
			to: "/orgs/$organization",
			params: { organization: org.id },
			search: true,
		});
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
