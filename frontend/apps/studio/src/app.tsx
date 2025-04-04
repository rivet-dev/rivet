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
	const { showDestroyed, tags } = router.state.location.search;

	set(actorFiltersAtom, {
		showDestroyed: showDestroyed ?? true,
		tags: Object.fromEntries(tags?.map((tag) => [tag[0], tag[1]]) || []),
	});

	set(currentActorIdAtom, router.state.location.search.actorId);
});

const effect2 = withAtomEffect(actorFiltersAtom, (get, set) => {
	const { tags, showDestroyed } = get(actorFiltersAtom);
	router.navigate({
		to: ".",
		search: (old) => ({
			...old,
			tags: Object.entries(tags).map(([key, value]) => [key, value]),
			showDestroyed,
		}),
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
