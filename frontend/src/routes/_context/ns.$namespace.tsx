import { createFileRoute, redirect, useNavigate } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { GettingStarted } from "@/app/getting-started";
import { SidebarlessHeader } from "@/app/layout";
import { NotFoundCard } from "@/app/not-found-card";
import { RouteLayout } from "@/app/route-layout";
import { useDialog } from "@/app/use-dialog";
import { ls } from "@/components";
import { deriveProviderFromMetadata } from "@/lib/data";
import { posthog } from "@/lib/posthog";
import {
	RECENT_NAMESPACES_KEY,
	recordRecentVisit,
} from "@/lib/recently-visited";

export const Route = createFileRoute("/_context/ns/$namespace")({
	context: ({ context, params }) =>
		match(context)
			.with({ __type: "engine" }, (ctx) => ({
				dataProvider: context.getOrCreateEngineNamespaceContext(
					ctx.dataProvider,
					params.namespace,
				),
			}))
			.otherwise(() => {
				throw new Error("Invalid context type for this route");
			}),
	beforeLoad: ({ params, search }) => {
		recordRecentVisit(RECENT_NAMESPACES_KEY, params.namespace);

		const s = search as unknown as {
			skipOnboarding?: boolean;
			onboardingSuccess?: boolean;
		};
		if (s.skipOnboarding) {
			ls.onboarding.skipWelcomeEngine(params.namespace);
			posthog.capture("onboarding_skipped", {
				namespace: params.namespace,
			});
			throw redirect({ to: ".", search: {} });
		}
		if (s.onboardingSuccess) {
			throw redirect({ to: ".", search: {} });
		}
	},
	loaderDeps(opts) {
		const s = opts.search as unknown as {
			skipOnboarding?: boolean;
			backendOnboardingSuccess?: boolean;
			onboardingSuccess?: boolean;
		};
		return {
			skipOnboarding: s.skipOnboarding,
			backendOnboardingSuccess: s.backendOnboardingSuccess,
			onboardingSuccess: s.onboardingSuccess,
		};
	},
	async loader({ params, deps, context }) {
		const d = deps as {
			skipOnboarding?: boolean;
			backendOnboardingSuccess?: boolean;
			onboardingSuccess?: boolean;
		};
		const isSkipped =
			ls.onboarding.getSkipWelcomeEngine(params.namespace) ||
			d.skipOnboarding;

		if (isSkipped === true) {
			return {
				dataProvider: context.dataProvider,
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
			dataProvider: context.dataProvider,
			displayOnboarding: !hasBackendConfigured && !hasActors,
			displayFrontendOnboarding: hasBackendConfigured && !hasActors,
			provider: runnerProvider,
		};
	},
	component: RouteComponent,
	notFoundComponent: () => <NotFoundCard />,
});

function RouteComponent() {
	const {
		displayOnboarding,
		displayFrontendOnboarding,
		provider: runnerProvider,
	} = Route.useLoaderData();
	const { provider } = Route.useSearch();
	const { namespace } = Route.useParams();

	if (displayOnboarding || displayFrontendOnboarding) {
		return (
			<>
				<SidebarlessHeader />
				<GettingStarted
					key={namespace}
					displayFrontendOnboarding={displayFrontendOnboarding}
					provider={provider || runnerProvider}
				/>
				<Modals />
			</>
		);
	}

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
