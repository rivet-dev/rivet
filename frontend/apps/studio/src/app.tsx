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
import { RouterProvider, createRouter } from "@tanstack/react-router";
import { Suspense } from "react";
import { routeTree } from "./routeTree.gen";
import { withAtomEffect } from "jotai-effect";
import {
	actorFiltersAtom,
	currentActorIdAtom,
	pickActorListFilters,
} from "@rivet-gg/components/actors";
import { useAtom } from "jotai";

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

const effect = withAtomEffect(actorFiltersAtom, (get, set) => {
	// set initial values
	const search = router.state.location.search;

	const filters = pickActorListFilters(search);

	set(actorFiltersAtom, filters);
	set(currentActorIdAtom, router.state.location.search.actorId);
});

const effect2 = withAtomEffect(actorFiltersAtom, (get, set) => {
	return router.subscribe("onResolved", (event) => {
		set(actorFiltersAtom, pickActorListFilters(event.toLocation.search));
		set(currentActorIdAtom, event.toLocation.search.actorId);
	});
});

function InnerApp() {
	useAtom(effect);
	useAtom(effect2);

	return <RouterProvider router={router} />;
}

export function App() {
	return (
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
	);
}
