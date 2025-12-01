import { createFileRoute, redirect } from "@tanstack/react-router";
import { RouteLayout } from "@/app/route-layout";

export const Route = createFileRoute("/onboarding/choose-organization")({
	component: RouteComponent,
	beforeLoad: async ({ context }) => {
		const org = await context.clerk.createOrganization({
			name: `${context.clerk.user?.firstName}'s Organization`,
		});

		await context.clerk.setActive({ organization: org.id });
		await context.clerk.session?.reload();

		throw redirect({
			to: "/orgs/$organization",
			params: { organization: org.id },
		});
	},
});

function RouteComponent() {
	return (
		<RouteLayout>
			<div className="bg-card h-full border my-2 mr-2 rounded-lg">
				<div className="mt-2 flex flex-col items-center justify-center h-full">
					<div className="w-full sm:w-96">
						Creating your organization...
					</div>
				</div>
			</div>
		</RouteLayout>
	);
}
