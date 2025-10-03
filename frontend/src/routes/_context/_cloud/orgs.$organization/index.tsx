import { createFileRoute, notFound, redirect } from "@tanstack/react-router";
import { match } from "ts-pattern";
import CreateProjectFrameContent from "@/app/dialogs/create-project-frame";
import { RouteError } from "@/app/route-error";
import { RouteLayout } from "@/app/route-layout";
import { Card, H2, Skeleton } from "@/components";

export const Route = createFileRoute("/_context/_cloud/orgs/$organization/")({
	loader: async ({ context, params }) => {
		return match(context)
			.with({ __type: "cloud" }, async () => {
				if (!context.clerk?.organization) {
					return;
				}
				const result = await context.queryClient.fetchInfiniteQuery(
					context.dataProvider.currentOrgProjectsQueryOptions(),
				);

				const firstProject = result.pages[0].projects[0];

				if (firstProject) {
					throw redirect({
						to: "/orgs/$organization/projects/$project",
						replace: true,

						params: {
							organization: params.organization,
							project: firstProject.name,
						},
					});
				}
			})
			.otherwise(() => {
				throw notFound();
			});
	},
	wrapInSuspense: true,
	pendingMinMs: 0,
	pendingMs: 0,
	pendingComponent: () => (
		<RouteLayout>
			<div className="bg-card h-full border my-2 mr-2 rounded-lg">
				<div className="max-w-5xl  mt-2 flex justify-between items-center px-6 py-4">
					<H2 className="mb-2">
						<Skeleton className="w-48 h-8" />
					</H2>
				</div>
				<p className="max-w-5xl mb-6 px-6 text-muted-foreground">
					<Skeleton className="w-full h-4" />
				</p>
				<hr className="mb-4" />
				<div className="p-4 px-6 max-w-5xl ">
					<Skeleton className="h-8 w-48 mb-4" />
					<div className="flex flex-wrap gap-2 my-4">
						<Skeleton className="min-w-48 h-auto min-h-28 rounded-md" />
						<Skeleton className="min-w-48 h-auto min-h-28 rounded-md" />
						<Skeleton className="min-w-48 h-auto min-h-28 rounded-md" />
					</div>
				</div>
			</div>
		</RouteLayout>
	),
	component: RouteComponent,
	errorComponent: RouteError,
});

function RouteComponent() {
	return (
		<RouteLayout>
			<div className="bg-card h-full border my-2 mr-2 rounded-lg">
				<div className="mt-2 flex flex-col items-center justify-center h-full">
					<Card className="min-w-96">
						<CreateProjectFrameContent />
					</Card>
				</div>
			</div>
		</RouteLayout>
	);
}
