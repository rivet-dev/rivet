import {
	createFileRoute,
	notFound,
	Outlet,
	useNavigate,
	useSearch,
} from "@tanstack/react-router";
import { match } from "ts-pattern";
import { useDialog } from "@/app/use-dialog";
import { ModalRenderer } from "@/components/modal-renderer";

export const Route = createFileRoute("/_context/_cloud")({
	component: RouteComponent,
	beforeLoad: () => {
		return match(__APP_TYPE__)
			.with("cloud", async () => {})
			.otherwise(() => {
				throw notFound();
			});
	},
});

function RouteComponent() {
	return (
		<>
			<Outlet />
			<ModalRenderer />
			<CloudModals />
		</>
	);
}

function CloudModals() {
	const navigate = useNavigate();
	const search = useSearch({ from: "/_context" });

	const CreateProjectDialog = useDialog.CreateProject.Dialog;
	const CreateNamespaceDialog = useDialog.CreateNamespace.Dialog;

	return (
		<>
			<CreateProjectDialog
				dialogProps={{
					open: search.modal === "create-project",
					// FIXME
					onOpenChange: (value: any) => {
						if (!value) {
							navigate({
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
			<CreateNamespaceDialog
				dialogProps={{
					open: search.modal === "create-ns",
					// FIXME
					onOpenChange: (value: any) => {
						if (!value) {
							navigate({
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
		</>
	);
}
