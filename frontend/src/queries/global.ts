import {
	CancelledError,
	type InfiniteData,
	MutationCache,
	QueryCache,
	QueryClient,
	type QueryKey,
	queryOptions,
} from "@tanstack/react-query";
import type { Rivet } from "@rivetkit/engine-api-full";
import { posthog } from "@/lib/posthog";
import { toast } from "@/components";
import { isRivetApiError } from "@/lib/errors";
import { modal } from "@/utils/modal-utils";
import { Changelog } from "./types";

const previousQueryCache = new QueryCache();

const queryCache = new QueryCache({
	onError(error, query) {
		// Silently ignore CancelledError - these are expected during navigation/unmount
		if (error instanceof CancelledError) {
			return;
		}

		if (
			query.meta?.mightRequireAuth &&
			"statusCode" in error &&
			error.statusCode === 403
		) {
			modal.open("ProvideEngineCredentials", { dismissible: false });
			return;
		}

		if (query.meta?.reportType) {
			posthog.capture(query.meta.reportType, {
				type: "error",
				error,
				queryKey: query.queryKey,
			});
		}

		if (query.meta?.statusCheck) {
			previousQueryCache.remove(query);
		}
	},
	onSuccess(data, query) {
		if (query.meta?.statusCheck) {
			if (!previousQueryCache.find(query)) {
				previousQueryCache.add(query);
				queryClient.invalidateQueries({
					predicate: (q) => q.state.error !== null,
				});
			}
		}

		if (query.meta?.actorsListPage1Poll) {
			const response = data as Rivet.ActorsListResponse;
			const targetKey = query.meta.actorsListTargetQueryKey as QueryKey;
			queryClient.setQueryData<InfiniteData<Rivet.ActorsListResponse>>(
				targetKey,
				(old) => {
					if (!old?.pages.length) return old;
					const existingIds = new Set(
						old.pages.flatMap((p) => p.actors.map((a) => a.actorId)),
					);
					const newActors = response.actors.filter(
						(a) => !existingIds.has(a.actorId),
					);
					if (!newActors.length) return old;
					return {
						...old,
						pages: [
							{
								...old.pages[0],
								actors: [...newActors, ...old.pages[0].actors],
							},
							...old.pages.slice(1),
						],
					};
				},
			);
		}
	},
});

const mutationCache = new MutationCache({
	onError(error, variables, context, mutation) {
		console.error(error);
		if (mutation.meta?.hideErrorToast) {
			return;
		}
		const description = isRivetApiError(error)
			? error.body.message
			: error.message;

		toast.error("Operation failed", {
			description,
		});
	},
});

export const changelogQueryOptions = () => {
	return queryOptions({
		queryKey: ["changelog", __APP_BUILD_ID__],
		staleTime: 1 * 60 * 60 * 1000, // 1 hour
		queryFn: async () => {
			const response = await fetch("https://rivet.dev/changelog.json");
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
		mutations: {
			retry: 0,
		},
	},
	queryCache,
	mutationCache,
});
