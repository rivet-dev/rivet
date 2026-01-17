import {
	createFileRoute,
	redirect,
	useNavigate,
	useSearch,
} from "@tanstack/react-router";
import { posthog } from "posthog-js";
import { createNamespaceContext } from "@/app/data-providers/cloud-data-provider";
import { GettingStarted } from "@/app/getting-started";
import { SidebarlessHeader } from "@/app/layout";
import { NotFoundCard } from "@/app/not-found-card";
import { RouteLayout } from "@/app/route-layout";
import { FullscreenLoading, ls } from "@/components";

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
			posthog.capture("onboarding_skipped");
			throw redirect({ to: ".", search: {} });
		}
		if (search.onboardingSuccess) {
			throw redirect({ to: ".", search: {} });
		}

		return {
			dataProvider: {
				...context.dataProvider,
				...createNamespaceContext({
					...context.dataProvider,
					namespace: params.namespace,
					engineNamespaceId: ns.access.engineNamespaceId,
					engineNamespaceName: ns.access.engineNamespaceName,
				}),
			},
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

		const [runnerNames, runnerConfigs] = await Promise.all([
			context.queryClient.fetchInfiniteQuery(
				context.dataProvider.runnerNamesQueryOptions(),
			),
			context.queryClient.fetchInfiniteQuery(
				context.dataProvider.runnerConfigsQueryOptions(),
			),
		]);

		const actors = await context.queryClient.fetchQuery(
			context.dataProvider.actorsCountQueryOptions(),
		);

		const hasRunnerNames = runnerNames.pages[0].names.length > 0;
		const hasRunnerConfigs =
			Object.entries(runnerConfigs.pages[0].runnerConfigs).length > 0;
		const hasActors = actors > 0;

		const displayOnboarding =
			!isSkipped &&
			((!hasRunnerNames && !hasRunnerConfigs) || !hasActors);

		const displayBackendOnboarding =
			!isSkipped && !hasRunnerNames && !hasRunnerConfigs;

		return { displayOnboarding, displayBackendOnboarding };
	},
	notFoundComponent: () => <NotFoundCard />,
	pendingMinMs: 0,
	pendingMs: 0,
	pendingComponent: FullscreenLoading,
});

function RouteComponent() {
	const { displayOnboarding, displayBackendOnboarding } =
		Route.useLoaderData();
	const { template, noTemplate } = Route.useSearch();

	if (displayOnboarding || displayBackendOnboarding) {
		return (
			<>
				<SidebarlessHeader />
				<GettingStarted
					template={template}
					noTemplate={noTemplate}
					displayOnboarding={displayOnboarding}
					displayBackendOnboarding={displayBackendOnboarding}
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
