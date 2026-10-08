import {
	createFileRoute,
	type ErrorComponentProps,
	notFound,
} from "@tanstack/react-router";
import z from "zod";
import { ClusterPage } from "@/app/byoc/cluster-page";
import {
	ClusterLoadError,
	PalletClusterError,
	PalletClusterPage,
} from "@/app/pallet/cluster-page";
import { RouteError } from "@/app/route-error";
import { RouteLayout } from "@/app/route-layout";
import { isAuthError, isNotFoundError } from "@/lib/errors";
import { features } from "@/lib/features";

export const Route = createFileRoute(
	"/_context/orgs/$organization/clusters/$cluster",
)({
	validateSearch: z.object({
		regions: z.array(z.string()).optional(),
	}),
	beforeLoad: async ({ context, params }) => {
		if (features.byoc) {
			const byocCluster = await context.queryClient
				.ensureQueryData(
					context.dataProvider.currentOrgClusterQueryOptions({
						cluster: params.cluster,
					}),
				)
				.catch((error: unknown) => {
					if (features.pallet && isNotFoundError(error)) return null;
					throw error;
				});
			if (byocCluster) {
				return { clusterKind: "byoc" as const };
			}
		}

		if (!features.pallet) {
			throw notFound();
		}

		const cluster = await context.queryClient
			.fetchQuery(
				context.dataProvider.palletClusterQueryOptions({
					cluster: params.cluster,
				}),
			)
			.catch((error: unknown) => {
				if (isAuthError(error)) throw error;
				throw new ClusterLoadError(params.cluster, {
					cause: error,
				});
			});
		if (!cluster) {
			throw notFound();
		}
		return { clusterKind: "pallet" as const };
	},
	loader: ({ context }) => ({ clusterKind: context.clusterKind }),
	component: RouteComponent,
	errorComponent: ClusterRouteError,
	pendingMinMs: 0,
	pendingMs: 0,
	pendingComponent: ClusterPagePending,
});

function RouteComponent() {
	const { cluster } = Route.useParams();
	const { clusterKind } = Route.useLoaderData();

	if (clusterKind === "pallet") {
		return (
			<RouteLayout>
				<PalletClusterPage cluster={cluster} />
			</RouteLayout>
		);
	}

	return (
		<RouteLayout>
			<ClusterPage cluster={cluster} />
		</RouteLayout>
	);
}

function ClusterRouteError(props: ErrorComponentProps) {
	if (props.error instanceof ClusterLoadError) {
		return (
			<RouteLayout>
				<PalletClusterError error={props.error} />
			</RouteLayout>
		);
	}
	return <RouteError {...props} />;
}

function ClusterPagePending() {
	return (
		<RouteLayout>
			{features.pallet ? (
				<PalletClusterPage.Skeleton />
			) : (
				<ClusterPage.Skeleton />
			)}
		</RouteLayout>
	);
}
