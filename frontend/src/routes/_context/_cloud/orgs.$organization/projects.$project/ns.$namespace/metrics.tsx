import { createFileRoute } from "@tanstack/react-router";
import { NamespaceMetricsPage } from "@/app/metrics/namespace-metrics-page";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace/metrics",
)({
	component: NamespaceMetricsPage,
});
