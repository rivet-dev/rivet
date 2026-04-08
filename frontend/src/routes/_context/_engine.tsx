import {
	createFileRoute,
	notFound,
	Outlet,
	useNavigate,
	useSearch,
} from "@tanstack/react-router";
import { useDialog } from "@/app/use-dialog";
import { features } from "@/lib/features";

export const Route = createFileRoute("/_context/_engine")({
	component: RouteComponent,
	beforeLoad: () => {
		if (features.multitenancy) {
			throw notFound();
		}
	},
});

function RouteComponent() {
	return (
		<>
			<Outlet />
			<EngineModals />
		</>
	);
}

function EngineModals() {
	const navigate = useNavigate();
	const search = useSearch({ from: "/_context" });

	const CreateNamespaceDialog = useDialog.CreateNamespace.Dialog;

	return (
		<CreateNamespaceDialog
			dialogProps={{
				open: search.modal === "create-ns",
				onOpenChange: (value) => {
					if (!value) {
						return navigate({
							to: ".",
							search: (old) => ({
								...old,
								modal: undefined,
							}),
						});
					}
				},
			}}
		/>
	);
}
