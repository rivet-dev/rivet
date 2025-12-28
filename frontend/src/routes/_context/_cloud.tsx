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
				await waitForClerk(context.clerk);
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
	const ConnectQuickVercelDialog = useDialog.ConnectQuickVercel.Dialog;
	const ConnectRailwayDialog = useDialog.ConnectRailway.Dialog;
	const ConnectQuickRailwayDialog = useDialog.ConnectQuickRailway.Dialog;
	const ConnectManualDialog = useDialog.ConnectManual.Dialog;
	const ConnectAwsDialog = useDialog.ConnectAws.Dialog;
	const ConnectGcpDialog = useDialog.ConnectGcp.Dialog;
	const ConnectHetznerDialog = useDialog.ConnectHetzner.Dialog;
	const EditProviderConfigDialog = useDialog.EditProviderConfig.Dialog;
	const DeleteConfigDialog = useDialog.DeleteConfig.Dialog;
	const ShowRunnerMetadataDialog = useDialog.ShowRunnerMetadata.Dialog;

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
			<ConnectQuickVercelDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
				dialogProps={{
					open: search.modal === "connect-q-vercel",
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
			<ConnectQuickRailwayDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
				dialogProps={{
					open: search.modal === "connect-q-railway",
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
			<ConnectManualDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
				dialogProps={{
					open: search.modal === "connect-custom",
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
			<ConnectAwsDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
				dialogProps={{
					open: search.modal === "connect-aws",
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
			<ConnectGcpDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
				dialogProps={{
					open: search.modal === "connect-gcp",
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
			<ConnectHetznerDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
				dialogProps={{
					open: search.modal === "connect-hetzner",
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
			<EditProviderConfigDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
				name={search.config}
				dc={search.dc}
				dialogProps={{
					open: search.modal === "edit-provider-config",
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
			<DeleteConfigDialog
				name={search.config}
				dialogProps={{
					open: search.modal === "delete-provider-config",
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
			<ShowRunnerMetadataDialog
				runnerId={search.runnerId}
				dialogProps={{
					open: search.modal === "show-runner-metadata",
					// FIXME
					onOpenChange: (value: any) => {
						if (!value) {
							navigate({
								to: ".",
								search: (old) => ({
									...old,
									modal: undefined,
									runnerId: undefined,
								}),
							});
						}
					},
				}}
			/>
		</>
	);
}
