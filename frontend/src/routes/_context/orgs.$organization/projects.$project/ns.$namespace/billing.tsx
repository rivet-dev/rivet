import { createFileRoute } from "@tanstack/react-router";
import { BillingPage } from "@/app/billing/billing-page";

export const Route = createFileRoute(
	"/_context/orgs/$organization/projects/$project/ns/$namespace/billing",
)({
	component: BillingPage,
});
