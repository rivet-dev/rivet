import {
	type RegisteredRouter,
	type RouteIds,
	useRouteContext,
} from "@tanstack/react-router";
import { match } from "ts-pattern";

export const useDataProvider = () =>
	useRouteContext({
		from: match(__APP_TYPE__)
			.with("cloud", () => {
				return "/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace" as const;
			})
			.with("engine", () => {
				return "/_context/_engine/ns/$namespace" as const;
			})
			.with("inspector", () => {
				return "/_context/_inspector" as const;
			})
			.exhaustive(),
	}).dataProvider;

export const useEngineDataProvider = () => {
	return useRouteContext({
		from: "/_context/_engine",
	}).dataProvider;
};

export const useInspectorDataProvider = () => {
	return useRouteContext({
		from: "/_context/_inspector",
	}).dataProvider;
};

type OnlyCloudRouteIds = Extract<
	RouteIds<RegisteredRouter["routeTree"]>,
	`/_context/_cloud/${string}`
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

export const useEngineCompatDataProvider = () => {
	return useRouteContext({
		from: match(__APP_TYPE__)
			.with("cloud", () => {
				return "/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace" as const;
			})
			.with("engine", () => {
				return "/_context/_engine/ns/$namespace" as const;
			})
			.otherwise(() => {
				throw new Error("Not in an engine-like context");
			}),
	}).dataProvider;
};
