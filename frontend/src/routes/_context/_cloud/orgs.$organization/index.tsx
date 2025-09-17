import { faChevronLeft, Icon } from "@rivet-gg/icons";
import {
	createFileRoute,
	Link,
	notFound,
	redirect,
} from "@tanstack/react-router";
import { match } from "ts-pattern";
import CreateProjectFrameContent from "@/app/dialogs/create-project-frame";
import { Logo } from "@/app/logo";
import { Card } from "@/components";

export const Route = createFileRoute("/_context/_cloud/orgs/$organization/")({
	beforeLoad: async ({ context, params }) => {
		return match(context)
			.with({ __type: "cloud" }, async () => {
				if (!context.clerk?.organization) {
					throw notFound();
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

	component: RouteComponent,
});

function RouteComponent() {
	return (
		<div className="flex flex-col gap-6 px-4 w-full mx-auto h-screen min-h-0 max-h-screen items-center justify-safe-center overflow-auto py-8">
			<div className="flex flex-col items-center gap-6">
				<div>
					<Link
						className="text-xs text-muted-foreground mb-4 block"
						from={Route.to}
						to="/orgs/$organization/projects"
					>
						<Icon icon={faChevronLeft} /> Organizations
					</Link>
					<Card className="w-full sm:w-96">
						<CreateProjectFrameContent />
					</Card>
				</div>
			</div>
		</div>
	);
}
