import { createFileRoute } from "@tanstack/react-router";
import { createNamespaceContext } from "@/app/data-providers/cloud-data-provider";
import { RouteLayout } from "@/app/route-layout";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace",
)({
	component: RouteComponent,
	beforeLoad: async ({ context, params }) => {
		if (context.__type !== "cloud") {
			throw new Error("Invalid context type for this route");
		}
		const ns = await context.queryClient.fetchQuery(
			context.dataProvider.currentProjectNamespaceQueryOptions({
				namespace: params.namespace,
			}),
		);

		return {
			dataProvider: {
				...context.dataProvider,
				...createNamespaceContext({
					...context.dataProvider,
					namespace: params.namespace,
					engineNamespaceId: ns.access.engineNamespaceId,
					engineNamespaceName: ns.access.engineNamespaceName,
				}),
			},
		};
	},
});

function RouteComponent() {
	return <RouteLayout />;
}
