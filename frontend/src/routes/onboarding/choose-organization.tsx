import { CreateOrganization, useOrganizationList } from "@clerk/clerk-react";
import { createFileRoute, Navigate } from "@tanstack/react-router";
import { RouteLayout } from "@/app/route-layout";

export const Route = createFileRoute("/onboarding/choose-organization")({
	component: RouteComponent,
});

function RouteComponent() {
	const {
		userMemberships: { data: userMemberships },
	} = useOrganizationList({ userMemberships: true });

	return (
		<RouteLayout>
			<div className="bg-card h-full border my-2 mr-2 rounded-lg">
				<div className="mt-2 flex flex-col items-center justify-center h-full">
					<div className="w-full sm:w-96">
						{userMemberships?.length ? (
							<Navigate
								to={`/orgs/$organization`}
								params={{
									organization:
										userMemberships[0].organization.id,
								}}
								replace
							/>
						) : null}
						<CreateOrganization
							hideSlug
							afterCreateOrganizationUrl={(org) =>
								`/orgs/${org.id}`
							}
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
	);
}
