import type { Clerk } from "@clerk/clerk-js";
import * as Sentry from "@sentry/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { createRouter, Link, RouterProvider } from "@tanstack/react-router";
import { Suspense } from "react";
import {
	Button,
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
	ConfigProvider,
	FullscreenLoading,
	getConfig,
	H2,
	ThirdPartyProviders,
	Toaster,
	TooltipProvider,
} from "@/components";
import { RouteLayout } from "./app/route-layout";
import { RootLayout } from "./components/layout";
import { clerk } from "./lib/auth";
import { queryClient } from "./queries/global";
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
		};
	}
}

export const router = createRouter({
	basepath: import.meta.env.BASE_URL,
	routeTree,
	context: {
		clerk:
			__APP_TYPE__ === "cloud" ? clerk : (undefined as unknown as Clerk),
		queryClient: queryClient,
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
			<div className="bg-card h-full border my-2 mr-2 rounded-lg">
				<div className="mt-2 flex flex-col items-center justify-center h-full">
					<div className="w-full sm:w-96">
						<Card>
							<CardHeader>
								<CardTitle className="flex items-center">
									404
								</CardTitle>
								<CardDescription>
									The page was not found
								</CardDescription>
							</CardHeader>
							<CardContent>
								<Button asChild variant="secondary">
									<Link to="/">Go home</Link>
								</Button>
							</CardContent>
						</Card>
					</div>
				</div>
			</div>
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
