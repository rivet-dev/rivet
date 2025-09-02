import { MutationCache, QueryCache, QueryClient } from "@tanstack/react-query";
import { toast } from "@/components";

const queryCache = new QueryCache();

const mutationCache = new MutationCache({
	onError(error, variables, context, mutation) {
		if (mutation.meta?.hideErrorToast) {
			return;
		}
		toast.error("Error occurred while performing the operation.", {
			description: error.message,
		});
	},
});

export const queryClient = new QueryClient({
	defaultOptions: {
		queries: {
			staleTime: 5 * 1000,
			gcTime: 60 * 1000,
			retry: 3,
			refetchOnWindowFocus: true,
			refetchOnReconnect: false,
		},
	},
	queryCache,
	mutationCache,
});
