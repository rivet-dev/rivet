import { createFileRoute, notFound } from "@tanstack/react-router";
import z from "zod";
import { ClusterPage } from "@/app/byoc/cluster-page";
import { RouteError } from "@/app/route-error";
import { RouteLayout } from "@/app/route-layout";
import { features } from "@/lib/features";

export const Route = createFileRoute(
	"/_context/orgs/$organization/clusters/$cluster",
)({
	validateSearch: z.object({
		regions: z.array(z.string()).optional(),
	}),
	beforeLoad: async ({ context, params }) => {
		if (!features.byoc) {
			throw notFound();
		}

		await context.queryClient.ensureQueryData(
			context.dataProvider.currentOrgClusterQueryOptions({
				cluster: params.cluster,
			}),
		);
	},
	component: RouteComponent,
	errorComponent: RouteError,
	pendingMinMs: 0,
	pendingMs: 0,
	pendingComponent: ClusterPagePending,
});

function RouteComponent() {
	const { cluster } = Route.useParams();
	return (
		<RouteLayout>
			<ClusterPage cluster={cluster} />
		</RouteLayout>
	);
}

function ClusterPagePending() {
	return (
		<RouteLayout>
			<ClusterPage.Skeleton />
		</RouteLayout>
	);
}
