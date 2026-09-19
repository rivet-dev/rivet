import { useMatch, useMatchRoute } from "@tanstack/react-router";
import { createContext, useContext } from "react";
import type {
	createGlobalContext as createGlobalCloudContext,
	createNamespaceContext as createNamespaceCloudContext,
	createOrganizationContext as createOrganizationCloudContext,
	createProjectContext as createProjectCloudContext,
} from "@/app/data-providers/cloud-data-provider";
import type {
	createGlobalContext as createGlobalEngineContext,
	createNamespaceContext as createNamespaceEngineContext,
} from "@/app/data-providers/engine-data-provider";
import { features } from "@/lib/features";

type EngineDataProvider = ReturnType<typeof createNamespaceEngineContext> &
	ReturnType<typeof createGlobalEngineContext>;

type CloudDataProvider = ReturnType<typeof createNamespaceCloudContext> &
	ReturnType<typeof createProjectCloudContext> &
	ReturnType<typeof createOrganizationCloudContext> &
	ReturnType<typeof createGlobalCloudContext>;

// Optional override for environments without a TanStack Router (inspector tab
// iframes). When provided, all useDataProvider variants short-circuit and
// return this value; otherwise they fall back to useLoaderData against the
// matching route. The dashboard never needs to set this; only the iframe
// runtime does, by reading the shell's provider off window.parent.
export const DataProviderContext = createContext<
	EngineDataProvider | CloudDataProvider | null
>(null);

// Data providers are derived synchronously in each route's `context()` and
// re-exported from its `loader`. Reads prefer loader data, per convention, and
// fall back to the match context so a still-pending match does not read as
// missing: with `pendingMs: 0` and an async `beforeLoad`, a param change swaps
// in a new match whose `loaderData` is undefined while chrome mounted above the
// route (top bar, settings drawer) keeps rendering against it.
//
// The cloud namespace route is the exception: it builds its provider in an
// async `beforeLoad`, and its match context carries the *project* provider
// until that resolves, so falling back there would silently hand out the wrong
// scope. It reads loader data only, and its callers gate on
// `useNamespaceDataProviderReady`.
const useEngineGlobalRouteDataProvider = () =>
	useMatch({
		from: "/_context",
		select: (match) =>
			match.loaderData?.dataProvider ?? match.context.dataProvider,
	});

const useEngineNamespaceRouteDataProvider = () =>
	useMatch({
		from: "/_context/ns/$namespace",
		select: (match) =>
			match.loaderData?.dataProvider ?? match.context.dataProvider,
	});

const useCloudOrganizationRouteDataProvider = () =>
	useMatch({
		from: "/_context/orgs/$organization",
		select: (match) =>
			match.loaderData?.dataProvider ?? match.context.dataProvider,
	});

const useCloudProjectRouteDataProvider = () =>
	useMatch({
		from: "/_context/orgs/$organization/projects/$project",
		select: (match) =>
			match.loaderData?.dataProvider ?? match.context.dataProvider,
	});

const useCloudNamespaceRouteDataProvider = () =>
	useMatch({
		from: "/_context/orgs/$organization/projects/$project/ns/$namespace",
		select: (match) => match.loaderData?.dataProvider as CloudDataProvider,
	});

export const useDataProvider = (): EngineDataProvider | CloudDataProvider => {
	const override = useContext(DataProviderContext);
	if (override) return override;
	if (features.platform) {
		// biome-ignore lint/correctness/useHookAtTopLevel: guarded by override above
		return useCloudNamespaceRouteDataProvider() as CloudDataProvider;
	}
	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
	return useEngineNamespaceRouteDataProvider() as EngineDataProvider;
};

export const useDataProviderCheck = () => {
	const matchRoute = useMatchRoute();
	// Fuzzy-match the project route so this passes on the project index page
	// (no namespace selected yet) as well as its nested namespace pages. The
	// project route and its descendants all carry a data provider in their
	// loader.
	return matchRoute({
		fuzzy: true,
		to: features.platform
			? "/orgs/$organization/projects/$project"
			: "/ns/$namespace",
	});
};

export const useEngineDataProvider = () => {
	return useEngineGlobalRouteDataProvider();
};

export const useEngineNamespaceDataProvider = () => {
	const override = useContext(DataProviderContext);
	if (override) return override as EngineDataProvider;
	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by override above
	return useEngineNamespaceRouteDataProvider();
};

export const useCloudDataProvider = () => {
	const override = useContext(DataProviderContext);
	if (override) return override as CloudDataProvider;
	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by override above
	return useCloudOrganizationRouteDataProvider();
};

export const useCloudProjectDataProvider = () => {
	const override = useContext(DataProviderContext);
	if (override) return override as CloudDataProvider;
	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by override above
	return useCloudProjectRouteDataProvider();
};

export const useCloudNamespaceDataProvider = () => {
	const override = useContext(DataProviderContext);
	if (override) return override as CloudDataProvider;
	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by override above
	return useCloudNamespaceRouteDataProvider();
};

export const useEngineCompatDataProvider = () => {
	const override = useContext(DataProviderContext);
	if (override) return override;
	if (features.platform) {
		// biome-ignore lint/correctness/useHookAtTopLevel: guarded by override above
		return useCloudNamespaceRouteDataProvider() as
			| EngineDataProvider
			| CloudDataProvider;
	}
	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by override above
	return useEngineNamespaceRouteDataProvider() as
		| EngineDataProvider
		| CloudDataProvider;
};

/**
 * Whether `useEngineCompatDataProvider` can currently be read.
 *
 * That hook takes its provider from the namespace route's loader, which is
 * absent while the route is still pending. The namespace route renders the top
 * bar from its `pendingComponent`, so chrome mounted there can run ahead of the
 * loader. `useDataProviderCheck` does not cover this: it fuzzy-matches the
 * *project* route, and matching a route says nothing about whether its loader
 * has resolved.
 */
export const useNamespaceDataProviderReady = () => {
	const override = useContext(DataProviderContext);
	const cloudMatch = useMatch({
		from: "/_context/orgs/$organization/projects/$project/ns/$namespace",
		shouldThrow: false,
	});
	const engineMatch = useMatch({
		from: "/_context/ns/$namespace",
		shouldThrow: false,
	});

	if (override) return true;
	return !!(features.platform
		? cloudMatch?.loaderData
		: engineMatch?.loaderData);
};
