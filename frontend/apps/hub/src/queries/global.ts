import { ls } from "@/lib/ls";
import { isRivetError } from "@/lib/utils";
import { RivetClient } from "@rivet-gg/api";
import { RivetClient as RivetEeClient } from "@rivet-gg/api-ee";
import { type APIResponse, type Fetcher, fetcher } from "@rivet-gg/api/core";
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
import { watchBlockingQueries } from "./watch";

declare module "@tanstack/react-query" {
	interface Register {
		queryMeta: {
			/**
			 * Injected by the watch function to indicate the index to watch for
			 * do not use this directly
			 */
			__watcher?: { index: string };
			/**
			 * If true, the query will be watched for a response
			 */
			watch?:
				| true
				| ((oldData: unknown, streamChunk: unknown) => unknown);

			/**
			 * Runs when the query is updated
			 */
			updateCache?: (
				// biome-ignore lint/suspicious/noExplicitAny: we don't know the shape of the data, it's up to the user to define it
				data: any,
				queryClient: QueryClient,
			) => Promise<void> | void;
		};
	}
}

const logout = async () => {
	await queryClient.cancelQueries();
	queryClient.clear();
	ls.remove("rivet-token");
	window.location.reload();
};

const queryCache = new QueryCache({
	onSuccess: async (data, query) => {
		if (query.meta?.updateCache) {
			await query.meta.updateCache(data, queryClient);
		}
	},
	onError: (error) => {
		if (isRivetError(error)) {
			if (
				error.body.code === "TOKEN_REVOKED" ||
				error.body.code === "TOKEN_INVALID"
			) {
				logout();
			}
		}
	},
});

const mutationCache = new MutationCache({
	onError(error, variables, context, mutation) {
		console.error(error);
		if (mutation.meta?.hideErrorToast) {
			return;
		}
		toast.error("Error occurred while performing the operation.", {
			description: isRivetError(error) ? error.body.message : undefined,
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

queryClient.setMutationDefaults(["identityToken"], {
	scope: { id: "identityToken" },
	gcTime: timing.minutes(15),
	mutationFn: () =>
		rivetClientTokeneless.auth.tokens.refreshIdentityToken({
			logout: false,
		}),
	onSuccess: async (data) => {
		ls.set("rivet-token", data);
	},
});

const tokenMutationObserver = new MutationObserver(queryClient, {
	mutationKey: ["identityToken"],
});

const clientOptions: RivetClient.Options = {
	environment: getConfig().apiUrl,
	fetcher: async <R = unknown>(
		args: Fetcher.Args,
	): Promise<APIResponse<R, Fetcher.Error>> => {
		const headers = args.headers || {};

		headers["X-Fern-Language"] = undefined;
		headers["X-Fern-Runtime"] = undefined;
		headers["X-Fern-Runtime-Version"] = undefined;

		const response = await fetcher<R>({
			...args,
			withCredentials: true,
			maxRetries: 0,
		});

		return response;
	},
	token: async () => {
		const result = ls.get("rivet-token");
		if (!result || new Date(result.exp).getTime() < Date.now()) {
			await tokenMutationObserver.mutate();
			return ls.get("rivet-token").token;
		}
		return result.token;
	},
};

export const rivetClientTokeneless = new RivetClient({
	environment: clientOptions.environment,
	fetcher: clientOptions.fetcher,
});
export const rivetClient = new RivetClient(clientOptions);
export const rivetEeClient = new RivetEeClient(clientOptions);

watchBlockingQueries(queryClient);

broadcastQueryClient({
	queryClient,
	broadcastChannel: "rivet-gg-hub",
});
