import { useLoaderData, useMatchRoute } from "@tanstack/react-router";
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

export const useDataProvider = (): EngineDataProvider | CloudDataProvider => {
	if (features.multitenancy) {
		// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
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
	return matchRoute({
		fuzzy: true,
		to: features.multitenancy
			? "/orgs/$organization/projects/$project/ns/$namespace"
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
	return useLoaderData({
		from: "/_context/ns/$namespace",
		select: (d) => d.dataProvider,
	});
};

export const useCloudDataProvider = () => {
	return useLoaderData({
		from: "/_context/orgs/$organization",
		select: (d) => d.dataProvider,
	});
};

export const useCloudProjectDataProvider = () => {
	return useLoaderData({
		from: "/_context/orgs/$organization/projects/$project",
		select: (d) => d.dataProvider,
	});
};

export const useCloudNamespaceDataProvider = () => {
	return useLoaderData({
		from: "/_context/orgs/$organization/projects/$project/ns/$namespace",
		select: (d) => d?.dataProvider,
	});
};

export const useEngineCompatDataProvider = () => {
	if (features.multitenancy) {
		// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
		return useLoaderData({
			from: "/_context/orgs/$organization/projects/$project/ns/$namespace",
			select: (d) => d.dataProvider,
		}) as EngineDataProvider | CloudDataProvider;
	}
	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
	return useLoaderData({
		from: "/_context/ns/$namespace",
		select: (d) => d.dataProvider,
	}) as EngineDataProvider | CloudDataProvider;
};
