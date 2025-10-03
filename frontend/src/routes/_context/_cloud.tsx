import {
	createFileRoute,
	notFound,
	Outlet,
	useNavigate,
	useSearch,
} from "@tanstack/react-router";
import { match } from "ts-pattern";
import { useDialog } from "@/app/use-dialog";
import { waitForClerk } from "@/lib/waitForClerk";

export const Route = createFileRoute("/_context/_cloud")({
	component: RouteComponent,
	beforeLoad: ({ context }) => {
		return match(context)
			.with({ __type: "cloud" }, async () => {
				return await waitForClerk(context.clerk);
			})
			.otherwise(() => {
				throw notFound();
			});
	},
});

function RouteComponent() {
	return (
		<>
			<Outlet />
			<CloudModals />
		</>
	);
}

function CloudModals() {
	const navigate = useNavigate();
	const search = useSearch({ from: "/_context" });

	const CreateProjectDialog = useDialog.CreateProject.Dialog;
	const CreateNamespaceDialog = useDialog.CreateNamespace.Dialog;
	const ConnectVercelDialog = useDialog.ConnectVercel.Dialog;
	const ConnectRailwayDialog = useDialog.ConnectRailway.Dialog;

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
			<ConnectVercelDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
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
				dialogContentProps={{
					className: "max-w-xl",
				}}
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
