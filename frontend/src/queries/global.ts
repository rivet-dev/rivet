import {
	MutationCache,
	QueryCache,
	QueryClient,
	queryOptions,
} from "@tanstack/react-query";
import { toast } from "@/components";
import { modal } from "@/utils/modal-utils";
import { Changelog } from "./types";

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

export const changelogQueryOptions = () => {
	return queryOptions({
		queryKey: ["changelog", __APP_BUILD_ID__],
		staleTime: 1 * 60 * 60 * 1000, // 1 hour
		queryFn: async () => {
			const response = await fetch(
				"https://rivet-site.vercel.app/changelog.json",
			);
			if (!response.ok) {
				throw new Error("Failed to fetch changelog");
			}
			const result = Changelog.parse(await response.json());
			return result;
		},
	});
};

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
