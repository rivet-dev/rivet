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
	const search = useSearch({ strict: false });

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
	const DeleteNamespaceDialog = useDialog.DeleteNamespace.Dialog;
	const DeleteProjectDialog = useDialog.DeleteProject.Dialog;
	const CreateOrganizationDialog = useDialog.CreateOrganization.Dialog;
	const UpsertDeploymentDialog = useDialog.UpsertDeployment.Dialog;

	return (
		<>
			<CreateProjectDialog
				organization={search?.organization}
				dialogProps={{
					open: search?.modal === "create-project",
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
			<CreateNamespaceDialog
				dialogProps={{
					open: search?.modal === "create-ns",
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
			<ConnectVercelDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
				dialogProps={{
					open: search?.modal === "connect-vercel",
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
			<ConnectQuickVercelDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
				dialogProps={{
					open: search?.modal === "connect-q-vercel",
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
			<ConnectQuickRailwayDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
				dialogProps={{
					open: search?.modal === "connect-q-railway",
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
			<ConnectRailwayDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
				dialogProps={{
					open: search?.modal === "connect-railway",
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
			<ConnectManualDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
				dialogProps={{
					open: search?.modal === "connect-custom",
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
			<ConnectAwsDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
				dialogProps={{
					open: search?.modal === "connect-aws",
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
			<ConnectGcpDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
				dialogProps={{
					open: search?.modal === "connect-gcp",
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
			<ConnectHetznerDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
				dialogProps={{
					open: search?.modal === "connect-hetzner",
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
			<EditProviderConfigDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
				name={search?.config}
				dc={search?.dc}
				dialogProps={{
					open: search?.modal === "edit-provider-config",
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
			<DeleteConfigDialog
				name={search?.config}
				dialogProps={{
					open: search?.modal === "delete-provider-config",
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
			<DeleteNamespaceDialog
				displayName={search?.displayName}
				dialogProps={{
					open: search?.modal === "delete-namespace",
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
			<DeleteProjectDialog
				displayName={search?.displayName}
				dialogProps={{
					open: search?.modal === "delete-project",
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
			<CreateOrganizationDialog
				dialogProps={{
					open: search?.modal === "create-organization",
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
			<UpsertDeploymentDialog
				namespace={search?.namespace}
				defaultImage={
					search?.repository && search?.tag
						? { repository: search.repository, tag: search.tag }
						: undefined
				}
				dialogProps={{
					open: search?.modal === "upsert-deployment",
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
		</>
	);
}
