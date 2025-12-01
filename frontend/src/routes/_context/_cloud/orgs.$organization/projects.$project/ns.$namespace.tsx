import {
	createFileRoute,
	useNavigate,
	useSearch,
} from "@tanstack/react-router";
import { createNamespaceContext } from "@/app/data-providers/cloud-data-provider";
import { NotFoundCard } from "@/app/not-found-card";
import { PendingRouteLayout, RouteLayout } from "@/app/route-layout";
import { useDialog } from "@/app/use-dialog";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace",
)({
	component: RouteComponent,
	beforeLoad: async ({ context, params }) => {
		if (context.__type !== "cloud") {
			throw new Error("Invalid context type for this route");
		}
		const ns = await context.queryClient.ensureQueryData(
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
	notFoundComponent: () => <NotFoundCard />,
	pendingMinMs: 0,
	pendingMs: 0,
	pendingComponent: PendingRouteLayout,
});

function RouteComponent() {
	return (
		<>
			<RouteLayout />
			<CloudNamespaceModals />
		</>
	);
}

function CloudNamespaceModals() {
	const navigate = useNavigate();
	const search = useSearch({ from: "/_context" });
	const StartWithTemplateDialog = useDialog.StartWithTemplate.Dialog;

	return (
		<StartWithTemplateDialog
			name={search.name}
			provider={search.provider}
			dialogProps={{
				open: search.modal === "start-with-template",
				onOpenChange: (value) => {
					if (!value) {
						return navigate({
							to: ".",
							search: (old) => ({
								...old,
								modal: undefined,
								name: undefined,
							}),
						});
					}
				},
			}}
		/>
	);
}
