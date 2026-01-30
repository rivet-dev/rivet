import { createFileRoute } from "@tanstack/react-router";
import { BillingPage } from "@/app/billing/billing-page";
import { RouteLayout } from "@/app/route-layout";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/billing",
)({
	component: () => (
		<RouteLayout>
			<BillingPage />
		</RouteLayout>
	),
});
