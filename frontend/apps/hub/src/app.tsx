import {
	ConfigProvider,
	FullscreenLoading,
	ThirdPartyProviders,
	Toaster,
	TooltipProvider,
	getConfig,
} from "@rivet-gg/components";
import { PageLayout } from "@rivet-gg/components/layout";
import * as Sentry from "@sentry/react";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { PersistQueryClientProvider } from "@tanstack/react-query-persist-client";
import {
	CatchBoundary,
	RouterProvider,
	createRouter,
} from "@tanstack/react-router";
import { Suspense } from "react";
import { LayoutedErrorComponent } from "./components/error-component";
import { AuthProvider, useAuth } from "./domains/auth/contexts/auth";
import { routeMasks } from "./lib/route-masks";
import { queryClient, queryClientPersister } from "./queries/global";
import { routeTree } from "./routeTree.gen";

declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router;
	}
	interface StaticDataRouteOption {
		layout?: "full" | "compact" | "onboarding" | "actors";
	}
}

export const router = createRouter({
	basepath: import.meta.env.BASE_URL,
	routeTree,
	routeMasks,
	context: {
		// biome-ignore lint/style/noNonNullAssertion: we know this will be defined
		auth: undefined!,
		queryClient,
	},
	// Since we're using React Query, we don't want loader calls to ever be stale
	// This will ensure that the loader is always called when the route is preloaded or visited
	defaultStaleTime: Number.POSITIVE_INFINITY,
	defaultPendingComponent: PageLayout.Root.Skeleton,
	defaultPreloadStaleTime: 0,
	defaultOnCatch: (error) => {
		Sentry.captureException(error);
	},
});

function InnerApp() {
	const auth = useAuth();
	return <RouterProvider router={router} context={{ auth }} />;
}

export function App({ cacheKey }: { cacheKey?: string }) {
	return (
		<PersistQueryClientProvider
			client={queryClient}
			persistOptions={{
				persister: queryClientPersister,
				buster: cacheKey,
			}}
		>
			<ConfigProvider value={getConfig()}>
				<ThirdPartyProviders>
					<Suspense fallback={<FullscreenLoading />}>
						<TooltipProvider>
							<CatchBoundary
								getResetKey={() => ""}
								errorComponent={LayoutedErrorComponent}
							>
								<AuthProvider>
									<InnerApp />
								</AuthProvider>
							</CatchBoundary>
						</TooltipProvider>
					</Suspense>

					<Toaster />
					<ReactQueryDevtools />
				</ThirdPartyProviders>
			</ConfigProvider>
		</PersistQueryClientProvider>
	);
}
