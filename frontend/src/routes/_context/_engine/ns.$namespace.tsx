import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { NotFoundCard } from "@/app/not-found-card";
import { RouteLayout } from "@/app/route-layout";
import { useDialog } from "@/app/use-dialog";

export const Route = createFileRoute("/_context/_engine/ns/$namespace")({
	context: ({ context, params }) => {
		return match(context)
			.with({ __type: "engine" }, (ctx) => {
				return {
					dataProvider: context.getOrCreateEngineNamespaceContext(
						ctx.dataProvider,
						params.namespace,
					),
				};
			})
			.otherwise(() => {
				throw new Error("Invalid context type for this route");
			});
	},
	component: RouteComponent,
	notFoundComponent: () => <NotFoundCard />,
});

function RouteComponent() {
	return (
		<>
			<Modals />
			<RouteLayout />
		</>
	);
}

function Modals() {
	const navigate = useNavigate();
	const search = Route.useSearch();

	const CreateActorDialog = useDialog.CreateActor.Dialog;

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

	return (
		<>
			<CreateActorDialog
				dialogProps={{
					open: search.modal === "create-actor",
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
					open: search.modal === "connect-vercel",
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
					open: search.modal === "connect-q-vercel",
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
					open: search.modal === "connect-q-railway",
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
					open: search.modal === "connect-railway",
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
					open: search.modal === "connect-custom",
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
					open: search.modal === "connect-aws",
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
					open: search.modal === "connect-gcp",
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
					open: search.modal === "connect-hetzner",
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
				name={search.config}
				dc={search.dc}
				dialogProps={{
					open: search.modal === "edit-provider-config",

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
				name={search.config}
				dialogProps={{
					open: search.modal === "delete-provider-config",

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
