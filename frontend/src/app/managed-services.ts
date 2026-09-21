import { useMutation } from "@tanstack/react-query";
import { toast } from "sonner";
import { useCloudNamespaceDataProvider } from "@/components/actors";
import { isRivetApiError } from "@/lib/errors";
import { queryClient } from "@/queries/global";

// Rivet runs every managed service for a namespace out of one dedicated compute
// pool. The pool existing is what "services are enabled" means for a namespace,
// so both the enable button and the service URL key off it.
export const MANAGED_SERVICES_POOL = "services";

export const MANAGED_SERVICES_POOL_CONFIG = {
	pool: MANAGED_SERVICES_POOL,
	displayName: "Services",
	image: { preset: { managedServices: {} } },
};

export function useManagedServicesPoolQueryOptions() {
	const dataProvider = useCloudNamespaceDataProvider();
	return dataProvider.currentNamespaceManagedPoolQueryOptions({
		pool: MANAGED_SERVICES_POOL,
		safe: true,
	});
}

export function useEnableManagedServicesMutation() {
	const dataProvider = useCloudNamespaceDataProvider();
	return useMutation({
		...dataProvider.upsertCurrentNamespaceManagedPoolMutationOptions(),
		onSuccess: async () => {
			await queryClient.invalidateQueries(
				dataProvider.currentNamespaceManagedPoolQueryOptions({
					pool: MANAGED_SERVICES_POOL,
					safe: true,
				}),
			);
		},
		// Replaces the generic mutation-cache toast rather than stacking on it.
		meta: { hideErrorToast: true },
		onError: (error) => {
			const reason = isRivetApiError(error)
				? error.body?.message
				: error.message;
			toast.error("Failed to enable services", {
				description:
					reason ||
					"The services pool could not be provisioned. Try again in a moment.",
			});
		},
	});
}
