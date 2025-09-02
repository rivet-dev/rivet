import { fetchEventSource } from "@microsoft/fetch-event-source";
import type {
	ActorId,
	Patch,
	RecordedRealtimeEvent,
} from "@rivetkit/core/inspector";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { applyPatch, compare } from "fast-json-patch";
import { useCallback, useEffect, useMemo } from "react";
import { useActor } from "../actor-queries-context";

export const useActorClearEventsMutation = (
	actorId: ActorId,
	options?: Parameters<typeof useMutation>[1],
) => {
	const queryClient = useQueryClient();
	const queries = useActor();
	return useMutation({
		...queries.actorClearEventsMutationOptions(actorId),
		onMutate: async () => {
			queryClient.setQueryData(
				queries.actorEventsQueryOptions(actorId).queryKey,
				() => ({ events: [] }),
			);
		},
		...options,
	});
};

export const useActorStatePatchMutation = (
	actorId: ActorId,
	options?: Parameters<typeof useMutation>[1],
) => {
	const queryClient = useQueryClient();
	const queries = useActor();
	return useMutation({
		// biome-ignore lint/suspicious/noExplicitAny: its really any
		mutationFn: async (data: any) => {
			const client = queries.createActorInspector(actorId);

			const oldStateQuery = queryClient.getQueryData(
				queries.actorStateQueryOptions(actorId).queryKey,
			);

			const oldState = oldStateQuery?.state;

			let response: Awaited<ReturnType<typeof client.state.$patch>>;

			if (!oldState || !isPatchable(data)) {
				response = await client.state.$patch({
					// its okay, we know the type
					// @ts-expect-error
					json: { replace: data },
				});
			} else {
				const patches = compare(oldState, data);
				response = await client.state.$patch({
					// its okay, we know the type
					// @ts-expect-error
					json: { patch: patches },
				});
			}

			if (!response.ok) {
				throw response;
			}
			return await response.json();
		},
		onSuccess: (data) => {
			queryClient.setQueryData(
				queries.actorStateQueryOptions(actorId).queryKey,
				data,
			);
		},
		...options,
	});
};

const getHeaders = (
	v:
		| Record<string, string>
		| (() => Record<string, string> | Promise<Record<string, string>>),
) => {
	if (typeof v === "function") {
		return v();
	}
	return v;
};

function useStream<T = unknown>(
	actorId: ActorId,
	onMessage: (data: T) => void,
	url: string,
	opts: { enabled: boolean } = { enabled: true },
) {
	const stableOnMessage = useCallback(onMessage, []);
	const queries = useActor();

	useEffect(() => {
		const controller = new AbortController();

		if (!opts.enabled) {
			controller.abort();
			return () => controller.abort();
		}

		async function establishConnection() {
			fetchEventSource(url, {
				signal: controller.signal,
				headers: await getHeaders(
					queries.createActorInspectorFetchConfiguration(actorId)
						?.headers || {},
				),
				onmessage: (event) => {
					const msg = JSON.parse(event.data);
					stableOnMessage(msg);
				},
				onclose: async () => {
					await new Promise((resolve) => setTimeout(resolve, 1000));
					controller.signal.throwIfAborted();
					establishConnection();
				},
			}).catch((error) => console.error(error));
		}

		establishConnection();
		return () => {
			controller.abort();
		};
	}, [url, actorId, opts.enabled, stableOnMessage]);
}

export const useActorStateStream = (
	actorId: ActorId,
	opts: { enabled: boolean } = { enabled: true },
) => {
	const queryClient = useQueryClient();
	const queries = useActor();

	useStream(
		actorId,
		useCallback(
			(data: unknown) => {
				queryClient.setQueryData(
					queries.actorStateQueryOptions(actorId).queryKey,
					() => ({ enabled: true, state: data }),
				);
			},
			[queryClient, actorId, queries],
		),
		useMemo(
			() =>
				queries.createActorInspector(actorId).state.stream.$url().href,
			[actorId, queries],
		),
		opts,
	);
};

export const useActorConnectionsStream = (actorId: ActorId) => {
	const queryClient = useQueryClient();
	const queries = useActor();

	useStream<RecordedRealtimeEvent[]>(
		actorId,
		useCallback(
			(data) => {
				queryClient.setQueryData(
					queries.actorConnectionsQueryOptions(actorId).queryKey,
					() => ({ enabled: true, connections: data }),
				);
			},
			[queryClient, actorId, queries],
		),
		useMemo(
			() =>
				queries.createActorInspector(actorId).connections.stream.$url()
					.href,
			[actorId, queries],
		),
	);
};

export const useActorEventsStream = (
	actorId: ActorId,
	opts: { enabled: boolean },
) => {
	const queryClient = useQueryClient();
	const queries = useActor();

	useStream(
		actorId,
		useCallback(
			(data: RecordedRealtimeEvent[]) => {
				queryClient.setQueryData(
					queries.actorEventsQueryOptions(actorId).queryKey,
					() => {
						return { events: data };
					},
				);
			},
			[queryClient, actorId, queries],
		),
		useMemo(
			() =>
				queries.createActorInspector(actorId).events.stream.$url().href,
			[actorId, queries],
		),
		opts,
	);
};

/**
 * Check if the object is patchable, i.e. if it is an object and not null.
 */
function isPatchable(data: unknown) {
	return typeof data === "object" && data !== null;
}
