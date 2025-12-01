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
	const ConnectQuickVercelDialog = useDialog.ConnectQuickVercel.Dialog;
	const ConnectRailwayDialog = useDialog.ConnectRailway.Dialog;
	const ConnectQuickRailwayDialog = useDialog.ConnectQuickRailway.Dialog;
	const ConnectManualDialog = useDialog.ConnectManual.Dialog;
	const ConnectAwsDialog = useDialog.ConnectAws.Dialog;
	const ConnectGcpDialog = useDialog.ConnectGcp.Dialog;
	const ConnectHetznerDialog = useDialog.ConnectHetzner.Dialog;
	const EditProviderConfigDialog = useDialog.EditProviderConfig.Dialog;
	const DeleteConfigDialog = useDialog.DeleteConfig.Dialog;
	const StartWithTemplateDialog = useDialog.StartWithTemplate.Dialog;

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
			<StartWithTemplateDialog
				template={search.template}
				dialogProps={{
					open: search.modal === "start-with-template",
					onOpenChange: (value) => {
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
