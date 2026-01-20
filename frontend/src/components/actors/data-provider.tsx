import {
	type RegisteredRouter,
	type RouteIds,
	useMatchRoute,
	useRouteContext,
} from "@tanstack/react-router";
import { match } from "ts-pattern";
import type {
	createNamespaceContext as createNamespaceCloudContext,
	createOrganizationContext as createOrganizationCloudContext,
	createProjectContext as createProjectCloudContext,
} from "@/app/data-providers/cloud-data-provider";
import type {
	createGlobalContext as createGlobalEngineContext,
	createNamespaceContext as createNamespaceEngineContext,
} from "@/app/data-providers/engine-data-provider";
import type { createGlobalContext as createGlobalInspectorContext } from "@/app/data-providers/inspector-data-provider";

export const useDataProvider = () => {
	return match(__APP_TYPE__)
		.with("cloud", () => {
			// biome-ignore lint/correctness/useHookAtTopLevel: runs only once
			return useRouteContext({
				from: "/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace",
				select: (ctx) => ctx.dataProvider,
			});
		})
		.with("engine", () => {
			// biome-ignore lint/correctness/useHookAtTopLevel: runs only once
			return useRouteContext({
				from: "/_context/_engine/ns/$namespace",
			}).dataProvider;
		})
		.with("inspector", () => {
			// we need to narrow down the context for inspector, because inspector does not have a unique route prefix
			return match(
				// biome-ignore lint/correctness/useHookAtTopLevel: runs only once
				useRouteContext({
					from: "/_context",
				}),
			)
				.with({ __type: "inspector" }, (ctx) => ctx.dataProvider)
				.otherwise(() => {
					throw new Error("Not in an inspector-like context");
				});
		})
		.exhaustive();
};

export const useDataProviderCheck = () => {
	const matchRoute = useMatchRoute();

	return matchRoute({
		fuzzy: true,
		to: match(__APP_TYPE__)
			.with("cloud", () => {
				return "/orgs/$organization/projects/$project/ns/$namespace" as const;
			})
			.with("engine", () => {
				return "/ns/$namespace" as const;
			})
			.with("inspector", () => {
				return "/" as const;
			})
			.otherwise(() => {
				throw new Error("Not in a valid context");
			}),
	});
};

export const useEngineDataProvider = () => {
	return useRouteContext({
		from: "/_context/_engine",
	}).dataProvider;
};

export const useEngineNamespaceDataProvider = () => {
	return useRouteContext({
		from: "/_context/_engine/ns/$namespace",
	}).dataProvider;
};

export const useInspectorDataProvider = () => {
	const context = useRouteContext({
		from: "/_context",
	});

	return match(context)
		.with({ __type: "inspector" }, (c) => c.dataProvider)
		.otherwise(() => {
			throw new Error("Not in an inspector-like context");
		});
};

type OnlyCloudRouteIds = Extract<
	RouteIds<RegisteredRouter["routeTree"]>,
	`/_context/_cloud/orgs/${string}`
>;

export const useCloudDataProvider = ({
	from = "/_context/_cloud/orgs/$organization",
}: {
	from?: OnlyCloudRouteIds;
} = {}) => {
	return useRouteContext({
		from,
	}).dataProvider;
};

export const useCloudNamespaceDataProvider = () => {
	return useRouteContext({
		from: "/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace",
	}).dataProvider;
};

export const useEngineCompatDataProvider = () => {
	const routePath = match(__APP_TYPE__)
		.with("cloud", () => {
			return "/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace" as const;
		})
		.with("engine", () => {
			return "/_context/_engine/ns/$namespace" as const;
		})
		.with("inspector", () => {
			return "/_context" as const;
		})
		.otherwise(() => {
			throw new Error("Not in an engine-like context");
		});

	return useRouteContext({
		from: routePath,
	}).dataProvider as
		| EngineDataProvider
		| CloudDataProvider
		| InspectorDataProvider;
};

type EngineDataProvider = ReturnType<typeof createNamespaceEngineContext> &
	ReturnType<typeof createGlobalEngineContext>;

type CloudDataProvider = ReturnType<typeof createNamespaceCloudContext> &
	ReturnType<typeof createProjectCloudContext> &
	ReturnType<typeof createOrganizationCloudContext> &
	ReturnType<typeof createGlobalInspectorContext>;

type InspectorDataProvider = ReturnType<typeof createGlobalInspectorContext>;
