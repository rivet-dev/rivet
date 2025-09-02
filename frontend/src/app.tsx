import * as Sentry from "@sentry/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { createRouter, RouterProvider } from "@tanstack/react-router";
import { Suspense, useEffect } from "react";
import {
	ConfigProvider,
	FullscreenLoading,
	getConfig,
	ThirdPartyProviders,
	Toaster,
	TooltipProvider,
} from "@/components";
import { PageLayout } from "@/components/layout";
import { queryClient } from "./queries/global";
import { routeTree } from "./routeTree.gen";
import { initializeShiftKeyTracking } from "./stores/shift-key-store";

declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router;
	}
}

export const router = createRouter({
	basepath: import.meta.env.BASE_URL,
	routeTree,
	defaultStaleTime: Number.POSITIVE_INFINITY,
	defaultPendingComponent: PageLayout.Root.Skeleton,
	defaultPreloadStaleTime: 0,
	defaultOnCatch: (error) => {
		Sentry.captureException(error);
	},
});

function InnerApp() {
	return <RouterProvider router={router} />;
}

export function App() {
	useEffect(() => {
		return initializeShiftKeyTracking();
	}, []);
	return (
		<QueryClientProvider client={queryClient}>
			<ConfigProvider value={getConfig()}>
				<ThirdPartyProviders>
					<Suspense fallback={<FullscreenLoading />}>
						<TooltipProvider>
							<InnerApp />
						</TooltipProvider>
					</Suspense>
				</ThirdPartyProviders>

				<Toaster />
			</ConfigProvider>

			<ReactQueryDevtools client={queryClient} />
		</QueryClientProvider>
	);
}
