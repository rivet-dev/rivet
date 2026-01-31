import type { UseSuspenseQueryOptions } from "@tanstack/react-query";
import { useSuspenseQuery } from "@tanstack/react-query";
import { match } from "ts-pattern";
import { getConfig } from "@/components";
import {
	useCloudNamespaceDataProvider,
	useEngineCompatDataProvider,
	useEngineNamespaceDataProvider,
} from "@/components/actors";
import { cloudEnv } from "@/lib/env";

export function usePublishableToken() {
	return match(__APP_TYPE__)
		.with("cloud", () => {
			// biome-ignore lint/correctness/useHookAtTopLevel: it's okay, its guarded by build constant
			return useSuspenseQuery(
				// biome-ignore lint/correctness/useHookAtTopLevel: it's okay, its guarded by build constant
				useCloudNamespaceDataProvider().publishableTokenQueryOptions() as UseSuspenseQueryOptions<string>,
			).data;
		})
		.with("engine", () => {
			// biome-ignore lint/correctness/useHookAtTopLevel: it's okay, its guarded by build constant
			return useSuspenseQuery(
				// biome-ignore lint/correctness/useHookAtTopLevel: it's okay, its guarded by build constant
				useEngineNamespaceDataProvider().engineAdminTokenQueryOptions() as UseSuspenseQueryOptions<string>,
			).data;
		})
		.otherwise(() => {
			throw new Error("Not in a valid context");
		});
}

export function useAdminToken() {
	return useSuspenseQuery(
		useEngineCompatDataProvider().engineAdminTokenQueryOptions() as UseSuspenseQueryOptions<string>,
	).data;
}

export const useEndpoint = () => {
	return match(__APP_TYPE__)
		.with("cloud", () => {
			return cloudEnv().VITE_APP_API_URL;
		})
		.with("engine", () => {
			return getConfig().apiUrl;
		})
		.otherwise(() => {
			throw new Error("Not in a valid context");
		});
};
