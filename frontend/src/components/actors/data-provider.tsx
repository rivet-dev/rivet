import {
	type RegisteredRouter,
	type RouteIds,
	useMatchRoute,
	useRouteContext,
} from "@tanstack/react-router";
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
		return useRouteContext({
			from: "/_context/orgs/$organization/projects/$project/ns/$namespace",
			select: (ctx) => ctx.dataProvider,
		}) as CloudDataProvider;
	}
	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
	return useRouteContext({
		from: "/_context/ns/$namespace",
	}).dataProvider as EngineDataProvider;
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
	return useRouteContext({
		from: "/_context",
	}).dataProvider;
};

export const useEngineNamespaceDataProvider = () => {
	return useRouteContext({
		from: "/_context/ns/$namespace",
	}).dataProvider;
};

type OnlyCloudRouteIds = Extract<
	RouteIds<RegisteredRouter["routeTree"]>,
	`/_context/orgs/${string}`
>;

export const useCloudDataProvider = ({
	from = "/_context/orgs/$organization",
}: {
	from?: OnlyCloudRouteIds;
} = {}) => {
	return useRouteContext({
		from,
	}).dataProvider;
};

export const useCloudProjectDataProvider = () => {
	return useRouteContext({
		from: "/_context/orgs/$organization/projects/$project",
	}).dataProvider;
};

export const useCloudNamespaceDataProvider = () => {
	return useRouteContext({
		from: "/_context/orgs/$organization/projects/$project/ns/$namespace",
	}).dataProvider;
};

export const useEngineCompatDataProvider = () => {
	const routePath = features.multitenancy
		? ("/_context/orgs/$organization/projects/$project/ns/$namespace" as const)
		: ("/_context/ns/$namespace" as const);

	return useRouteContext({
		from: routePath,
	}).dataProvider as EngineDataProvider | CloudDataProvider;
};
