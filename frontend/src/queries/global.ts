import { MutationCache, QueryCache, QueryClient } from "@tanstack/react-query";
import { toast } from "@/components";
import { modal } from "@/utils/modal-utils";

const queryCache = new QueryCache({
	onError(error, query) {
		if (
			query.meta?.mightRequireAuth &&
			"statusCode" in error &&
			error.statusCode === 403
		) {
			modal.open("ProvideEngineCredentials");
			return;
		}
	},
});

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
