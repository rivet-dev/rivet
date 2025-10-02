import {
	createFileRoute,
	notFound,
	Outlet,
	useNavigate,
	useSearch,
} from "@tanstack/react-router";
import { match } from "ts-pattern";
import { useDialog } from "@/app/use-dialog";

export const Route = createFileRoute("/_context/_engine")({
	component: RouteComponent,
	beforeLoad: () => {
		return match(__APP_TYPE__)
			.with("engine", () => {})
			.otherwise(() => {
				throw notFound();
			});
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

	const ConnectVercelDialog = useDialog.ConnectVercel.Dialog;
	const ConnectRailwayDialog = useDialog.ConnectRailway.Dialog;

	return (
		<>
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
			<ConnectVercelDialog
				dialogProps={{
					open: search.modal === "connect-vercel",
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
			<ConnectRailwayDialog
				dialogProps={{
					open: search.modal === "connect-railway",
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
