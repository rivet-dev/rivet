import * as Sentry from "@sentry/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { createRouter, RouterProvider } from "@tanstack/react-router";
import { Suspense } from "react";
import { NotFoundCard } from "@/app/not-found-card";
import { RouteLayout } from "@/app/route-layout";
import {
	ConfigProvider,
	FullscreenLoading,
	getConfig,
	ThirdPartyProviders,
	Toaster,
	TooltipProvider,
} from "@/components";
import { queryClient } from "@/queries/global";
import { getOrCreateInspectorContext } from "./app/data-providers/cache";
import { routeTree } from "./routeTree.gen";

declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router;
	}
}

declare module "@tanstack/react-query" {
	interface Register {
		queryMeta: {
			mightRequireAuth?: boolean;
			statusCheck?: boolean;
			reportType?: string;
		};
	}
}

export const router = createRouter({
	basepath: import.meta.env.BASE_URL,
	routeTree,
	context: {
		queryClient: queryClient,
		getOrCreateInspectorContext,
	},
	defaultPreloadStaleTime: 0,
	defaultGcTime: 0,
	defaultPreloadGcTime: 0,
	defaultStaleTime: Infinity,
	scrollRestoration: true,
	defaultPendingMinMs: 300,
	defaultPendingComponent: FullscreenLoading,
	defaultOnCatch: (error) => {
		console.error("Router caught an error:", error);
		Sentry.captureException(error);
	},
	defaultNotFoundComponent: () => (
		<RouteLayout>
			<NotFoundCard />
		</RouteLayout>
	),
});

function InnerApp() {
	return <RouterProvider router={router} />;
}

export function App() {
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
