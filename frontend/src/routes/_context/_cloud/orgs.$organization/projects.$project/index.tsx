import { faChevronLeft, faChevronRight, faHome, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import {
	createFileRoute,
	Link,
	notFound,
	redirect,
} from "@tanstack/react-router";
import { match } from "ts-pattern";
import CreateNamespacesFrameContent from "@/app/dialogs/create-namespace-frame";
import { Card, Skeleton } from "@/components";

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
		<div className="flex flex-col gap-6 px-4 w-full mx-auto h-screen min-h-0 max-h-screen items-center justify-safe-center overflow-auto py-8">
			<div className="flex flex-col items-center gap-6">
				<div>
					<Breadcrumbs />
					<Card className="w-full sm:w-96">
						<CreateNamespacesFrameContent />
					</Card>
				</div>
			</div>
		</div>
	);
}

function Breadcrumbs() {
	const { data: orgData } = useQuery(
		Route.useRouteContext().dataProvider.organizationQueryOptions({
			org: Route.useParams().organization,
		}),
	);
	const { data } = useQuery(
		Route.useRouteContext().dataProvider.currentProjectQueryOptions(),
	);
	return (
		<div className="text-xs text-muted-foreground mb-2 flex gap-1 items-center">
			<Link from={Route.to} to="/orgs" className="hover:underline">
				<Icon icon={faHome} />
			</Link>
			<Icon icon={faChevronRight} />
			<span className="max-w-32 truncate">
				{orgData?.name || <Skeleton className="h-4 w-16" />}
			</span>
			<Icon icon={faChevronRight} />
			<Link
				from={Route.to}
				to="/orgs/$organization/projects"
				className="hover:underline"
			>
				Projects
			</Link>
			<Icon icon={faChevronRight} />
			<span className="text-foreground font-medium">
				{data?.name || <Skeleton className="h-4 w-16" />}
			</span>
		</div>
	);
}
