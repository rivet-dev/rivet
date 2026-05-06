import type { UseSuspenseQueryOptions } from "@tanstack/react-query";
import { useSuspenseQuery } from "@tanstack/react-query";
import { getConfig } from "@/components";
import {
	useCloudNamespaceDataProvider,
	useEngineCompatDataProvider,
	useEngineNamespaceDataProvider,
} from "@/components/actors";
import { cloudEnv } from "@/lib/env";
import { features } from "@/lib/features";

export function usePublishableToken(): string | null {
	if (features.platform) {
		// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
		return useSuspenseQuery(
			// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
			useCloudNamespaceDataProvider().publishableTokenQueryOptions() as UseSuspenseQueryOptions<string>,
		).data;
	}
	// Enterprise (ACL without platform API) has no publishable-token endpoint;
	// users mint these out-of-band via RBAC config. Plain OSS has no auth.
	return null;
}

export function useAdminToken() {
	return useSuspenseQuery(
		useEngineCompatDataProvider().engineAdminTokenQueryOptions() as UseSuspenseQueryOptions<string>,
	).data;
}

export const useEndpoint = () => {
	if (features.platform) {
		return cloudEnv().VITE_APP_API_URL;
	}
	return getConfig().apiUrl;
};
