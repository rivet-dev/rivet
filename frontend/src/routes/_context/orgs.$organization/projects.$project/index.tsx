import { createFileRoute, notFound, redirect } from "@tanstack/react-router";
import CreateNamespacesFrameContent from "@/app/dialogs/create-namespace-frame";
import { RouteLayout } from "@/app/route-layout";
import { Card } from "@/components";
import { features } from "@/lib/features";

export const Route = createFileRoute(
	"/_context/orgs/$organization/projects/$project/",
)({
	beforeLoad: async ({ context, params }) => {
		if (!features.multitenancy) {
			throw notFound();
		}

		const result = await context.queryClient.fetchInfiniteQuery(
			context.dataProvider.currentProjectNamespacesQueryOptions(),
		);

		const firstNamespace = result.pages[0].namespaces[0];

		if (firstNamespace) {
			throw redirect({
				to: "/orgs/$organization/projects/$project/ns/$namespace",
				replace: true,
				search: true,
				params: {
					organization: params.organization,
					project: params.project,
					namespace: firstNamespace.name,
				},
			});
		}
	},
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<RouteLayout>
			<div className="bg-card h-full border my-2 mr-2 rounded-lg">
				<div className="mt-2 flex flex-col items-center justify-center h-full">
					<Card className="min-w-96">
						<CreateNamespacesFrameContent />
					</Card>
				</div>
			</div>
		</RouteLayout>
	);
}
