import { createFileRoute, notFound, redirect } from "@tanstack/react-router";
import { match } from "ts-pattern";
import CreateNamespacesFrameContent from "@/app/dialogs/create-namespace-frame";
import { Card } from "@/components";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/",
)({
	beforeLoad: ({ context, params }) => {
		return match(__APP_TYPE__)
			.with("cloud", async () => {
				if (!context.clerk?.organization) {
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
						reloadDocument: true,
						params: {
							organization: params.organization,
							project: params.project,
							namespace: firstNamespace.name,
						},
					});
				}
			})
			.otherwise(() => {
				throw notFound();
			});
	},
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<div className="flex flex-col gap-6 w-full mx-auto h-screen items-center justify-center overflow-auto">
			<Card className="w-full max-w-md">
				<CreateNamespacesFrameContent />
			</Card>
		</div>
	);
}
