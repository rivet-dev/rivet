import { createFileRoute } from "@tanstack/react-router";
import { MetricsPage } from "@/app/metrics/metrics-page";
import { RouteLayout } from "@/app/route-layout";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/metrics",
)({
	component: () => (
		<RouteLayout>
			<MetricsPage />
		</RouteLayout>
	),
});
