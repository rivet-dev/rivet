import { getConfig, timing, toast } from "@rivet-gg/components";
import { broadcastQueryClient } from "@tanstack/query-broadcast-client-experimental";
import { createSyncStoragePersister } from "@tanstack/query-sync-storage-persister";
import {
	MutationCache,
	MutationObserver,
	QueryCache,
	QueryClient,
} from "@tanstack/react-query";
import superjson from "superjson";

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
			gcTime: 1000 * 60 * 60 * 24,
			retry: 2,
			refetchOnWindowFocus: false,
			refetchOnReconnect: false,
		},
	},
	queryCache,
	mutationCache,
});

export const queryClientPersister = createSyncStoragePersister({
	storage: window.localStorage,
	serialize: superjson.stringify,
	deserialize: superjson.parse,
});

broadcastQueryClient({
	queryClient,
	broadcastChannel: "rivet-gg-hub",
});
