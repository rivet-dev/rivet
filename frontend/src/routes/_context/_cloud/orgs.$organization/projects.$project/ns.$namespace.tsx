import type { Rivet } from "@rivet-gg/cloud";
import {
	createFileRoute,
	redirect,
	useNavigate,
	useSearch,
} from "@tanstack/react-router";
import { posthog } from "posthog-js";
import { GettingStarted } from "@/app/getting-started";
import { SidebarlessHeader } from "@/app/layout";
import { NotFoundCard } from "@/app/not-found-card";
import { RouteLayout } from "@/app/route-layout";
import { useDialog } from "@/app/use-dialog";
import { FullscreenLoading, ls } from "@/components";
import { deriveProviderFromMetadata } from "@/lib/data";
import { isRivetApiError } from "@/lib/errors";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace",
)({
	component: RouteComponent,
	beforeLoad: async ({ context, params, search }) => {
		if (context.__type !== "cloud") {
			throw new Error("Invalid context type for this route");
		}

		let ns: Rivet.NamespacesGetResponse.Namespace;
		try {
			ns = await context.queryClient.ensureQueryData(
				context.dataProvider.currentProjectNamespaceQueryOptions({
					namespace: params.namespace,
				}),
			);
		} catch (error) {
			if (isRivetApiError(error) && error.statusCode === 404) {
				throw redirect({
					to: "/orgs/$organization/projects/$project",
					params,
				});
			}
			throw error;
		}

		if (search.skipOnboarding) {
			ls.onboarding.skipWelcome(params.project, params.namespace);
			posthog.capture("onboarding_skipped", {
				project: params.project,
				namespace: params.namespace,
			});
			throw redirect({ to: ".", search: {} });
		}
		if (search.onboardingSuccess) {
			throw redirect({ to: ".", search: {} });
		}

		return {
			dataProvider: context.getOrCreateCloudNamespaceContext(
				context.dataProvider,
				params.namespace,
				ns.access.engineNamespaceName,
				ns.access.engineNamespaceId,
			),
		};
	},
	loaderDeps(opts) {
		return {
			skipOnboarding: opts.search.skipOnboarding,
			backendOnboardingSuccess: opts.search.backendOnboardingSuccess,
			onboardingSuccess: opts.search.onboardingSuccess,
		};
	},
	async loader({ params, deps, context }) {
		const isSkipped =
			ls.onboarding.getSkipWelcome(params.project, params.namespace) ||
			deps.skipOnboarding;

		const namespace = await context.queryClient.fetchQuery(
			context.dataProvider.currentProjectNamespaceQueryOptions({
				namespace: params.namespace,
			}),
		);

		if (namespace.displayName !== "Production" || isSkipped === true) {
			return {
				displayOnboarding: false,
				displayFrontendOnboarding: false,
			};
		}

		const [runnerNames, runnerConfigs] = await Promise.all([
			context.queryClient.fetchInfiniteQuery(
				context.dataProvider.runnerNamesQueryOptions(),
			),
			context.queryClient.fetchInfiniteQuery(
				context.dataProvider.runnerConfigsQueryOptions(),
			),
		]);

		const runnerProvider = runnerConfigs.pages
			.flatMap((page) =>
				Object.values(page.runnerConfigs).flatMap((config) =>
					Object.values(config.datacenters).map((dc) =>
						deriveProviderFromMetadata(dc.metadata),
					),
				),
			)
			.find((provider) => provider !== undefined);

		const actors = await context.queryClient.fetchQuery(
			context.dataProvider.actorsCountQueryOptions(),
		);

		const hasRunnerNames = runnerNames.pages[0].names.length > 0;
		const hasRunnerConfigs =
			Object.entries(runnerConfigs.pages[0].runnerConfigs).length > 0;
		const hasActors = actors > 0;

		const hasBackendConfigured = hasRunnerNames || hasRunnerConfigs;

		return {
			displayOnboarding: !hasBackendConfigured && !hasActors,
			displayFrontendOnboarding: hasBackendConfigured && !hasActors,
			provider: runnerProvider,
		};
	},
	notFoundComponent: () => <NotFoundCard />,
	pendingMinMs: 0,
	pendingMs: 0,
	pendingComponent: FullscreenLoading,
});

function RouteComponent() {
	const {
		displayOnboarding,
		displayFrontendOnboarding,
		provider: runnerProvider,
	} = Route.useLoaderData();
	const { provider } = Route.useSearch();
	const { project, namespace } = Route.useParams();

	if (displayOnboarding || displayFrontendOnboarding) {
		return (
			<>
				<SidebarlessHeader />
				<GettingStarted
					key={`${project}-${namespace}`}
					displayFrontendOnboarding={displayFrontendOnboarding}
					provider={provider || runnerProvider}
				/>

				<CloudNamespaceModals />
			</>
		);
	}

	return (
		<>
			<RouteLayout />
			<CloudNamespaceModals />
		</>
	);
}

function CloudNamespaceModals() {
	const navigate = useNavigate();
	const search = useSearch({ strict: false });
	const CreateNamespaceDialog = useDialog.CreateNamespace.Dialog;
	const ConnectRivetDialog = useDialog.ConnectRivet.Dialog;
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
	const UpsertDeploymentDialog = useDialog.UpsertDeployment.Dialog;

	return (
		<>
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
			<ConnectRivetDialog
				dialogContentProps={{
					className: "max-w-xl",
				}}
				dialogProps={{
					open: search?.modal === "connect-rivet",
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
		</>
	);
}
