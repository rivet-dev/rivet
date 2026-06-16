import { createFileRoute } from "@tanstack/react-router";
import { NamespaceBillingPage } from "@/app/billing/billing-page";

export const Route = createFileRoute(
	"/_context/orgs/$organization/projects/$project/ns/$namespace/billing",
)({
	component: NamespaceBillingPage,
});
