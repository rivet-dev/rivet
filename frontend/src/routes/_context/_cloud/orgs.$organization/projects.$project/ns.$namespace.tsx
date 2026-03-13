import { createFileRoute, redirect } from "@tanstack/react-router";
import { posthog } from "posthog-js";
import { GettingStarted } from "@/app/getting-started";
import { SidebarlessHeader } from "@/app/layout";
import { NotFoundCard } from "@/app/not-found-card";
import { RouteLayout } from "@/app/route-layout";
import { FullscreenLoading, ls } from "@/components";
import { deriveProviderFromMetadata } from "@/lib/data";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace",
)({
	component: RouteComponent,
	beforeLoad: async ({ context, params, search }) => {
		if (context.__type !== "cloud") {
			throw new Error("Invalid context type for this route");
		}
		const ns = await context.queryClient.ensureQueryData(
			context.dataProvider.currentProjectNamespaceQueryOptions({
				namespace: params.namespace,
			}),
		);

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
			context.dataProvider.currentNamespaceQueryOptions(),
		);

		if (namespace.displayName !== "Production" || isSkipped === true) {
			return {
				displayOnboarding: false,
				displayBackendOnboarding: false,
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

		const hasManagedPoolRunner = runnerProvider === "rivet";
		const actors = await context.queryClient.fetchQuery(
			context.dataProvider.actorsCountQueryOptions(),
		);

		const hasRunnerNames = runnerNames.pages[0].names.length > 0;
		const hasRunnerConfigs =
			Object.entries(runnerConfigs.pages[0].runnerConfigs).length > 0;
		const hasActors = actors > 0;

		let displayOnboarding =
			(!hasRunnerNames && !hasRunnerConfigs) || !hasActors;

		let displayBackendOnboarding = !hasRunnerNames && !hasRunnerConfigs;

		if (hasManagedPoolRunner) {
			const managedPool = await context.queryClient.fetchQuery(
				context.dataProvider.currentNamespaceManagedPoolQueryOptions({
					pool: "default",
					safe: true,
				}),
			);

			const hasImage = !!managedPool?.config.image;
			const isReady = managedPool?.status === "ready";

			displayOnboarding = (!hasImage && !isReady) || !hasActors;
			displayBackendOnboarding = hasImage && isReady;
		}

		return {
			displayOnboarding,
			displayBackendOnboarding,
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
		displayBackendOnboarding,
		provider: runnerProvider,
	} = Route.useLoaderData();
	const { provider } = Route.useSearch();

	if (displayOnboarding || displayBackendOnboarding) {
		return (
			<>
				<SidebarlessHeader />
				<GettingStarted
					displayOnboarding={displayOnboarding}
					displayBackendOnboarding={displayBackendOnboarding}
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
	return null;
}
