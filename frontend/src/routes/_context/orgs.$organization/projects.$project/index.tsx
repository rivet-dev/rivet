import {
	createFileRoute,
	notFound,
	useNavigate,
	useSearch,
} from "@tanstack/react-router";
import { NamespacesGrid } from "@/app/namespaces-grid";
import { RouteLayout } from "@/app/route-layout";
import { useDialog } from "@/app/use-dialog";
import { features } from "@/lib/features";

export const Route = createFileRoute(
	"/_context/orgs/$organization/projects/$project/",
)({
	beforeLoad: async ({ context }) => {
		if (!features.multitenancy) {
			throw notFound();
		}

		// Prefetch namespaces so the grid renders without a flash.
		await context.queryClient.prefetchInfiniteQuery(
			context.dataProvider.currentProjectNamespacesQueryOptions(),
		);
	},
	component: RouteComponent,
});

function RouteComponent() {
	const { organization, project } = Route.useParams();
	return (
		<RouteLayout>
			<NamespacesGrid organization={organization} project={project} />
			<ProjectModals />
		</RouteLayout>
	);
}

function ProjectModals() {
	const navigate = useNavigate();
	const search = useSearch({ strict: false });
	const CreateNamespaceDialog = useDialog.CreateNamespace.Dialog;
	return (
		<CreateNamespaceDialog
			dialogProps={{
				open: search?.modal === "create-ns",
				onOpenChange: (value) => {
					if (!value) {
						return navigate({
							to: ".",
							search: (old) => ({ ...old, modal: undefined }),
						});
					}
				},
			}}
		/>
	);
}
