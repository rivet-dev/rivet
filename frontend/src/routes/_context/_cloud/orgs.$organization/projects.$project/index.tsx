import { createFileRoute, notFound, redirect } from "@tanstack/react-router";
import { match } from "ts-pattern";
import CreateNamespacesFrameContent from "@/app/dialogs/create-namespace-frame";
import { Logo } from "@/app/logo";
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
		<div className="flex flex-col gap-6 px-4 w-full mx-auto h-screen items-center justify-center overflow-auto">
			<div className="flex flex-col items-center gap-6">
				<Logo className="h-10 mb-4" />
				<Card className="w-full sm:w-96">
					<CreateNamespacesFrameContent />
				</Card>
			</div>
		</div>
	);
}
