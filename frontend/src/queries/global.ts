import type { Rivet } from "@rivetkit/engine-api-full";
import { createAsyncStoragePersister } from "@tanstack/query-async-storage-persister";
import {
	CancelledError,
	defaultShouldDehydrateQuery,
	type InfiniteData,
	MutationCache,
	QueryCache,
	QueryClient,
	type QueryKey,
	queryOptions,
} from "@tanstack/react-query";
import {
	type PersistQueryClientOptions,
	persistQueryClientRestore,
	persistQueryClientSubscribe,
} from "@tanstack/react-query-persist-client";
import { toast } from "@/components";
import { isAuthError, isRivetApiError } from "@/lib/errors";
import { posthog } from "@/lib/posthog";
import { modal } from "@/utils/modal-utils";
import { Changelog } from "./types";

const previousQueryCache = new QueryCache();

const queryCache = new QueryCache({
	onError(error, query) {
		// Silently ignore CancelledError - these are expected during navigation/unmount
		if (error instanceof CancelledError) {
			return;
		}

		if (query.meta?.mightRequireAuth && isAuthError(error)) {
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
						old.pages.flatMap((p) =>
							p.actors.map((a) => a.actorId),
						),
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
	onError(error, _variables, _context, mutation) {
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

// changelog.json media URLs are absolute assets.rivet.dev URLs on current
// entries but were site-relative paths historically. Resolve both shapes to
// absolute URLs here so consumers can pass them straight to src attributes.
const resolveChangelogMediaUrl = (url: string) =>
	new URL(url, "https://www.rivet.dev/").toString();

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
			return result.map((entry) => ({
				...entry,
				images: entry.images.map((image) => ({
					...image,
					url: resolveChangelogMediaUrl(image.url),
				})),
				authors: entry.authors.map((author) => ({
					...author,
					avatar: {
						...author.avatar,
						url: resolveChangelogMediaUrl(author.avatar.url),
					},
				})),
			}));
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

// Persist a curated allowlist of queries to localStorage so cache-first route
// loaders (such as the namespace onboarding gate) can resolve from cache across
// full page reloads instead of blocking on the network. Only queries tagged
// with `meta.persist` are stored, keeping large and sensitive payloads (actor
// data, tokens, inspector state) out of localStorage. The build id busts the
// persisted cache on every deploy so a schema change can never rehydrate stale
// data.
const queryCachePersister = createAsyncStoragePersister({
	storage: typeof window !== "undefined" ? window.localStorage : undefined,
	key: "rivet-query-cache",
});

const persistOptions: Omit<PersistQueryClientOptions, "queryClient"> = {
	persister: queryCachePersister,
	maxAge: 24 * 60 * 60 * 1000,
	buster: __APP_BUILD_ID__,
	dehydrateOptions: {
		shouldDehydrateQuery: (query) =>
			defaultShouldDehydrateQuery(query) && query.meta?.persist === true,
	},
};

// Rehydrates the persisted cache, then keeps it in sync. Must be awaited before
// the router mounts so loaders see restored data on first paint.
export async function restoreQueryCache() {
	if (typeof window === "undefined") return;
	await persistQueryClientRestore({ queryClient, ...persistOptions });
	persistQueryClientSubscribe({ queryClient, ...persistOptions });
}
