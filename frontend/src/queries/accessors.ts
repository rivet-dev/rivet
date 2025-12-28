import { useSuspenseQuery } from "@tanstack/react-query";
import { match } from "ts-pattern";
import { getConfig } from "@/components";
import {
	useCloudNamespaceDataProvider,
	useEngineNamespaceDataProvider,
} from "@/components/actors";
import { cloudEnv } from "@/lib/env";

export function usePublishableToken() {
	return match(__APP_TYPE__)
		.with("cloud", () => {
			// biome-ignore lint/correctness/useHookAtTopLevel: it's okay, its guarded by build constant
			return useSuspenseQuery(
				// biome-ignore lint/correctness/useHookAtTopLevel: it's okay, its guarded by build constant
				useCloudNamespaceDataProvider().publishableTokenQueryOptions(),
			).data;
		})
		.with("engine", () => {
			// biome-ignore lint/correctness/useHookAtTopLevel: it's okay, its guarded by build constant
			return useSuspenseQuery(
				// biome-ignore lint/correctness/useHookAtTopLevel: it's okay, its guarded by build constant
				useEngineNamespaceDataProvider().engineAdminTokenQueryOptions(),
			).data;
		})
		.otherwise(() => {
			throw new Error("Not in a valid context");
		});
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
