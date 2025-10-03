import { CreateOrganization } from "@clerk/clerk-react";
import { createFileRoute } from "@tanstack/react-router";
import { RouteLayout } from "@/app/route-layout";

export const Route = createFileRoute(
	"/onboarding/choose-organization",
)({
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<RouteLayout>
			<div className="bg-card h-full border my-2 mr-2 rounded-lg">
				<div className="mt-2 flex flex-col items-center justify-center h-full">
					<div className="w-full sm:w-96">
						<CreateOrganization
							hideSlug
							appearance={{
								variables: {
									colorBackground: "hsl(var(--card))",
								},
							}}
						/>
					</div>
				</div>
			</div>
		</RouteLayout>
	)
}
