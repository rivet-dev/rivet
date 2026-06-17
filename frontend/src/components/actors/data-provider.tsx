import { useLoaderData, useMatchRoute } from "@tanstack/react-router";
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

export const useDataProvider = (): EngineDataProvider | CloudDataProvider => {
	const override = useContext(DataProviderContext);
	if (override) return override;
	if (features.platform) {
		// biome-ignore lint/correctness/useHookAtTopLevel: guarded by override above
		return useLoaderData({
			from: "/_context/orgs/$organization/projects/$project/ns/$namespace",
			select: (d) => d.dataProvider,
		}) as CloudDataProvider;
	}
	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
	return useLoaderData({
		from: "/_context/ns/$namespace",
		select: (d) => d.dataProvider,
	}) as EngineDataProvider;
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
	return useLoaderData({
		from: "/_context",
		select: (d) => d.dataProvider,
	});
};

export const useEngineNamespaceDataProvider = () => {
	const override = useContext(DataProviderContext);
	if (override) return override as EngineDataProvider;
	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by override above
	return useLoaderData({
		from: "/_context/ns/$namespace",
		select: (d) => d.dataProvider,
	});
};

export const useCloudDataProvider = () => {
	const override = useContext(DataProviderContext);
	if (override) return override as CloudDataProvider;
	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by override above
	return useLoaderData({
		from: "/_context/orgs/$organization",
		select: (d) => d.dataProvider,
	});
};

export const useCloudProjectDataProvider = () => {
	const override = useContext(DataProviderContext);
	if (override) return override as CloudDataProvider;
	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by override above
	return useLoaderData({
		from: "/_context/orgs/$organization/projects/$project",
		select: (d) => d.dataProvider,
	});
};

export const useCloudNamespaceDataProvider = () => {
	const override = useContext(DataProviderContext);
	if (override) return override as CloudDataProvider;
	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by override above
	return useLoaderData({
		from: "/_context/orgs/$organization/projects/$project/ns/$namespace",
		select: (d) => d?.dataProvider,
	});
};

export const useEngineCompatDataProvider = () => {
	const override = useContext(DataProviderContext);
	if (override) return override;
	if (features.platform) {
		// biome-ignore lint/correctness/useHookAtTopLevel: guarded by override above
		return useLoaderData({
			from: "/_context/orgs/$organization/projects/$project/ns/$namespace",
			select: (d) => d.dataProvider,
		}) as EngineDataProvider | CloudDataProvider;
	}
	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by override above
	return useLoaderData({
		from: "/_context/ns/$namespace",
		select: (d) => d.dataProvider,
	}) as EngineDataProvider | CloudDataProvider;
};
