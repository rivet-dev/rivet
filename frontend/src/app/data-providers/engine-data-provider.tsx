import { type Rivet, RivetClient } from "@rivetkit/engine-api-full";
import { type Fetcher, fetcher } from "@rivetkit/engine-api-full/core";
import {
	infiniteQueryOptions,
	type MutationKey,
	mutationOptions,
	type QueryKey,
	queryOptions,
} from "@tanstack/react-query";
import { KV_KEYS } from "rivetkit/client";
import z from "zod";
import { getConfig, ls } from "@/components";
import type { ActorId } from "@/components/actors";
import { engineEnv } from "@/lib/env";
import { convertStringToId } from "@/lib/utils";
import { noThrow, shouldRetryAllExpect403 } from "@/queries/utils";
import {
	type ActorQueryOptions,
	ActorQueryOptionsSchema,
	createDefaultGlobalContext,
	RECORDS_PER_PAGE,
} from "./default-data-provider";

const mightRequireAuth = __APP_TYPE__ === "engine";

export type CreateNamespace = {
	displayName: string;
};

export type Namespace = {
	id: string;
	name: string;
	displayName: string;
	createdAt: string;
};

export function createClient(
	baseUrl = engineEnv().VITE_APP_API_URL,
	opts: { token: (() => string) | string | (() => Promise<string>) },
	fetcherArgs: Partial<Fetcher.Args> = {},
) {
	return new RivetClient({
		baseUrl: () => baseUrl,
		environment: "",
		...opts,
		fetcher: async (args) => {
			Object.keys(args.headers || {}).forEach((key) => {
				if (key.toLowerCase().startsWith("x-fern-")) {
					delete args.headers?.[key];
				}
			});
			return await fetcher({ ...args, ...fetcherArgs });
		},
	});
}

export const createGlobalContext = (opts: {
	engineToken: (() => string) | string | (() => Promise<string>);
}) => {
	const client = createClient(engineEnv().VITE_APP_API_URL, {
		token: opts.engineToken,
	});
	return {
		client,
		...opts,
		namespacesQueryOptions() {
			return infiniteQueryOptions({
				queryKey: ["namespaces"] as any,
				initialPageParam: undefined as string | undefined,
				queryFn: async ({ pageParam }) => {
					const data = await client.namespaces.list({
						limit: RECORDS_PER_PAGE,
						cursor: pageParam ?? undefined,
					});
					return {
						...data,
						namespaces: data.namespaces.map((ns) => ({
							id: ns.namespaceId,
							displayName: ns.displayName,
							name: ns.name,
							createdAt: new Date(ns.createTs).toISOString(),
						})),
					};
				},
				getNextPageParam: (lastPage) => {
					if (lastPage.namespaces.length < RECORDS_PER_PAGE) {
						return undefined;
					}
					return lastPage.pagination.cursor;
				},
				select: (data) => data.pages.flatMap((page) => page.namespaces),
				retry: shouldRetryAllExpect403,
				throwOnError: noThrow,
				meta: {
					mightRequireAuth,
				},
			});
		},
		namespaceQueryOptions(name: string | undefined) {
			return queryOptions({
				queryKey: ["namespace", name] as any,
				enabled: !!name,
				queryFn: async () => {
					const data = await client.namespaces.list({
						name,
					});
					return data.namespaces[0];
				},
			});
		},
		createNamespaceMutationOptions(opts: {
			onSuccess?: (data: Namespace) => void;
		}) {
			return {
				...opts,
				mutationKey: ["namespaces"],
				mutationFn: async (data: CreateNamespace) => {
					const response = await client.namespaces.create({
						displayName: data.displayName,
						name: convertStringToId(data.displayName),
					});

					return {
						id: response.namespace.namespaceId,
						name: response.namespace.name,
						displayName: response.namespace.displayName,
						createdAt: new Date(
							response.namespace.createTs,
						).toISOString(),
					};
				},
			};
		},
	};
};

export const createNamespaceContext = ({
	namespace,
	client,
	...parent
}: { namespace: string } & ReturnType<typeof createGlobalContext>) => {
	const def = createDefaultGlobalContext();
	const dataProvider = {
		...def,
		endpoint: engineEnv().VITE_APP_API_URL,
		features: {
			canCreateActors: true,
			canDeleteActors: true,
		},
		datacentersQueryOptions() {
			return infiniteQueryOptions({
				...def.datacentersQueryOptions(),
				enabled: true,
				queryKey: [
					{ namespace },
					...def.datacentersQueryOptions().queryKey,
				] as QueryKey,
				queryFn: async () => {
					const data = await client.datacenters.list();
					return data;
				},
				retry: shouldRetryAllExpect403,
				throwOnError: noThrow,
				meta: {
					mightRequireAuth,
				},
			});
		},
		datacenterQueryOptions(name: string | undefined) {
			return queryOptions({
				...def.datacenterQueryOptions(name),
				queryKey: [
					{ namespace },
					...def.datacenterQueryOptions(name).queryKey,
				],
				queryFn: async ({ client }) => {
					const regions = await client.ensureInfiniteQueryData(
						this.datacentersQueryOptions(),
					);

					for (const page of regions.pages) {
						for (const region of page.datacenters) {
							if (region.name === name) {
								return region;
							}
						}
					}

					throw new Error(`Region not found: ${name}`);
				},
				retry: shouldRetryAllExpect403,
				throwOnError: noThrow,
				meta: {
					mightRequireAuth,
				},
			});
		},
		actorQueryOptions(actorId: ActorId) {
			return queryOptions({
				...def.actorQueryOptions(actorId),
				queryKey: [
					{ namespace },
					...def.actorQueryOptions(actorId).queryKey,
				],
				enabled: true,
				queryFn: async () => {
					const data = await client.actorsList({
						actorIds: actorId as string,
						namespace,
					});

					if (!data.actors[0]) {
						throw new Error("Actor not found");
					}

					return data.actors[0];
				},
				retry: shouldRetryAllExpect403,
				throwOnError: noThrow,
				meta: {
					mightRequireAuth,
				},
			});
		},
		actorsQueryOptions(opts: ActorQueryOptions) {
			return infiniteQueryOptions({
				...def.actorsQueryOptions(opts),
				queryKey: [
					{ namespace },
					...def.actorsQueryOptions(opts).queryKey,
				],
				enabled: true,
				initialPageParam: undefined,
				queryFn: async ({ pageParam, queryKey: [, , _opts] }) => {
					const { success, data: opts } =
						ActorQueryOptionsSchema.safeParse(_opts || {});

					if (
						(opts?.n?.length === 0 || !opts?.n) &&
						(opts?.filters?.id?.value?.length === 0 ||
							!opts?.filters?.id?.value ||
							opts?.filters.key?.value?.length === 0 ||
							!opts?.filters.key?.value)
					) {
						// If there are no names specified, we can return an empty result
						return {
							actors: [],
							pagination: {
								cursor: undefined,
							},
						};
					}

					const data = await client.actorsList({
						namespace,
						cursor: pageParam ?? undefined,
						actorIds: opts?.filters?.id?.value?.join(","),
						key: opts?.filters?.key?.value?.join(","),
						includeDestroyed:
							success &&
							(opts?.filters?.showDestroyed?.value.includes(
								"true",
							) ||
								opts?.filters?.showDestroyed?.value.includes(
									"1",
								)),
						limit: opts?.limit ?? RECORDS_PER_PAGE,
						name: opts?.filters?.id?.value
							? undefined
							: opts?.n?.join(","),
					});

					return data;
				},
				getNextPageParam: (lastPage) => {
					if (lastPage.actors.length < RECORDS_PER_PAGE) {
						return undefined;
					}
					return lastPage.pagination?.cursor;
				},
				retry: shouldRetryAllExpect403,
				throwOnError: noThrow,
				meta: {
					mightRequireAuth,
				},
			});
		},
		buildsQueryOptions() {
			return infiniteQueryOptions({
				...def.buildsQueryOptions(),
				queryKey: [{ namespace }, ...def.buildsQueryOptions().queryKey],
				enabled: true,
				queryFn: async ({ pageParam }) => {
					const data = await client.actorsListNames({
						namespace,
						cursor: pageParam ?? undefined,
						limit: RECORDS_PER_PAGE,
					});

					return data;
				},
				getNextPageParam: (lastPage) => {
					if (Object.keys(lastPage.names).length < RECORDS_PER_PAGE) {
						return undefined;
					}
					return lastPage.pagination?.cursor;
				},
				retry: shouldRetryAllExpect403,
				throwOnError: noThrow,
				meta: {
					mightRequireAuth,
				},
			});
		},
		createActorMutationOptions() {
			return mutationOptions({
				...def.createActorMutationOptions(),
				mutationKey: [namespace, "actors"] as MutationKey,
				mutationFn: async (data) => {
					const response = await client.actorsCreate({
						namespace,
						name: data.name,
						key: data.key,
						datacenter: data.datacenter,
						crashPolicy: data.crashPolicy,
						runnerNameSelector: data.runnerNameSelector,
						input: JSON.stringify(data.input),
					});

					return response.actor.actorId;
				},
				onSuccess: () => {},
				throwOnError: noThrow,
				retry: shouldRetryAllExpect403,
				meta: {
					mightRequireAuth,
				},
			});
		},
		actorDestroyMutationOptions(actorId: ActorId) {
			return mutationOptions({
				...def.actorDestroyMutationOptions(actorId),
				throwOnError: noThrow,
				retry: shouldRetryAllExpect403,
				meta: {
					mightRequireAuth,
				},
				mutationFn: async () => {
					await client.actorsDelete(actorId, { namespace });
				},
			});
		},
		runnerHealthCheckQueryOptions(opts: {
			runnerUrl: string;
			headers: Record<string, string>;
		}) {
			return queryOptions({
				queryKey: ["runner", "healthcheck", opts] as QueryKey,
				enabled: !!opts.runnerUrl,
				queryFn: async () => {
					const healthCheck = (url: string) =>
						client.runnerConfigsServerlessHealthCheck({
							url: url,
							headers: opts.headers,
							namespace,
						});

					const url = opts.runnerUrl.replace(/\/+$/, "");
					if (!url.endsWith("/api/rivet")) {
						const res = await healthCheck(`${url}/api/rivet`);

						if ("success" in res) {
							return {
								url: `${url}/api/rivet`,
								success: res.success,
							};
						}
					}

					const res = await healthCheck(url);
					if ("success" in res) {
						return { url: url, success: res.success };
					}

					return { url: url, failure: res.failure };
				},
			});
		},
		actorInspectorTokenQueryOptions(actorId: ActorId) {
			return queryOptions({
				queryKey: [
					{ namespace },
					"actors",
					actorId,
					"inspector-token",
				] as QueryKey,
				enabled: !!actorId,
				retry: 0,
				queryFn: async () => {
					const response = await client.actorsKvGet(
						actorId,
						KV_KEYS.INSPECTOR_TOKEN
							// @ts-expect-error
							.toBase64(),
						{ namespace },
					);

					if (!response.value) {
						throw new Error("Inspector token not found");
					}

					return atob(response.value);
				},
			});
		},
		metadataQueryOptions() {
			return queryOptions({
				queryKey: [{ namespace }, "metadata"] as QueryKey,
				queryFn: async () => {
					return client.metadata.get();
				},
			});
		},
	};

	return {
		engineNamespace: namespace,
		engineToken: parent.engineToken,
		...dataProvider,
		runnersQueryOptions() {
			return infiniteQueryOptions({
				queryKey: [{ namespace }, "runners"] as QueryKey,
				initialPageParam: undefined as string | undefined,
				queryFn: async ({ pageParam }) => {
					const data = await client.runners.list({
						namespace,
						cursor: pageParam ?? undefined,
						limit: RECORDS_PER_PAGE,
					});
					return data;
				},
				getNextPageParam: (lastPage) => {
					if (lastPage.runners.length < RECORDS_PER_PAGE) {
						return undefined;
					}
					return lastPage.pagination.cursor;
				},
				select: (data) => data.pages.flatMap((page) => page.runners),
				retry: shouldRetryAllExpect403,
				meta: {
					mightRequireAuth,
				},
			});
		},
		runnerNamesQueryOptions() {
			return infiniteQueryOptions({
				queryKey: [{ namespace }, "runner", "names"] as QueryKey,
				initialPageParam: undefined as string | undefined,
				queryFn: async ({ pageParam }) => {
					const data = await client.runners.listNames({
						namespace,
						cursor: pageParam ?? undefined,
						limit: RECORDS_PER_PAGE,
					});
					return data;
				},
				getNextPageParam: (lastPage) => {
					if (lastPage.names.length < RECORDS_PER_PAGE) {
						return undefined;
					}
					return lastPage.pagination.cursor;
				},
				select: (data) => data.pages.flatMap((page) => page.names),
				retry: shouldRetryAllExpect403,
				throwOnError: noThrow,
				meta: {
					mightRequireAuth,
				},
			});
		},
		runnerQueryOptions(opts: { namespace: string; runnerId: string }) {
			return queryOptions({
				queryKey: [opts.namespace, "runner", opts.runnerId] as QueryKey,
				enabled: !!opts.runnerId,
				queryFn: async () => {
					const data = await client.runners.list({
						namespace: opts.namespace,
						runnerIds: opts.runnerId,
					});

					if (!data.runners[0]) {
						throw new Error("Runner not found");
					}
					return data.runners[0];
				},
				throwOnError: noThrow,
				retry: shouldRetryAllExpect403,
				meta: {
					mightRequireAuth,
				},
			});
		},
		runnerByNameQueryOptions(opts: { runnerName: string | undefined }) {
			return queryOptions({
				queryKey: [
					{ namespace },
					"runner",
					opts.runnerName,
				] as QueryKey,
				enabled: !!opts.runnerName,
				queryFn: async () => {
					const data = await client.runners.list({
						namespace,
						name: opts.runnerName,
					});
					if (!data.runners[0]) {
						throw new Error("Runner not found");
					}
					return data.runners[0];
				},
				retry: shouldRetryAllExpect403,
				meta: {
					mightRequireAuth,
				},
			});
		},
		upsertRunnerConfigMutationOptions(
			opts: {
				onSuccess?: (data: Rivet.RunnerConfigsUpsertResponse) => void;
			} = {},
		) {
			return mutationOptions({
				...opts,
				mutationKey: ["runner-config"] as QueryKey,
				mutationFn: async ({
					name,
					config,
				}: {
					name: string;
					config: Record<string, Rivet.RunnerConfig>;
				}) => {
					const response = await client.runnerConfigsUpsert(name, {
						namespace,
						datacenters: config,
					});
					return response;
				},
				retry: shouldRetryAllExpect403,
				meta: {
					mightRequireAuth,
				},
			});
		},
		deleteRunnerConfigMutationOptions(
			opts: { onSuccess?: (data: void) => void } = {},
		) {
			return mutationOptions({
				...opts,
				mutationKey: ["runner-config", "delete"] as QueryKey,
				mutationFn: async (name: string) => {
					await client.runnerConfigsDelete(name, { namespace });
				},
				retry: shouldRetryAllExpect403,
				meta: {
					mightRequireAuth,
				},
			});
		},
		runnerConfigsQueryOptions(opts?: {
			variant?: Rivet.RunnerConfigVariant;
		}) {
			return infiniteQueryOptions({
				queryKey: [
					{ namespace },
					"runners",
					"configs",
					opts,
				] as QueryKey,
				initialPageParam: undefined as string | undefined,
				queryFn: async ({ pageParam }) => {
					const response = await client.runnerConfigsList({
						namespace,
						cursor: pageParam ?? undefined,
						limit: RECORDS_PER_PAGE,
						variant: opts?.variant,
					});

					return response;
				},

				select: (data) =>
					data.pages.flatMap((page) =>
						Object.entries(page.runnerConfigs),
					),
				getNextPageParam: (lastPage) => {
					if (
						Object.values(lastPage.runnerConfigs).length <
						RECORDS_PER_PAGE
					) {
						return undefined;
					}
					return lastPage.pagination.cursor;
				},

				retryDelay: 50_000,
				retry: shouldRetryAllExpect403,
				meta: {
					mightRequireAuth,
				},
			});
		},

		runnerConfigQueryOptions(opts: {
			name: string | undefined;
			variant?: Rivet.RunnerConfigVariant;
		}) {
			return queryOptions({
				queryKey: [
					{ namespace },
					"runners",
					"config",
					opts,
				] as QueryKey,
				enabled: !!opts.name,
				queryFn: async () => {
					const response = await client.runnerConfigsList({
						namespace,
						runnerNames: opts.name,
						variant: opts.variant,
					});

					const config = response.runnerConfigs[opts.name!];

					if (!config) {
						throw new FetchError(
							"Provider Config not found",
							"The specified provider configuration could not be found.",
						);
					}

					return config;
				},
				retry: shouldRetryAllExpect403,
				meta: {
					mightRequireAuth,
				},
			});
		},
		engineAdminTokenQueryOptions() {
			return queryOptions({
				staleTime: 1000,
				gcTime: 1000,
				queryKey: [{ namespace }, "tokens", "engine-admin"] as QueryKey,
				queryFn: async () => {
					return (ls.engineCredentials.get(getConfig().apiUrl) ||
						"") as string;
				},
				meta: {
					mightRequireAuth,
				},
			});
		},

		actorsCountQueryOptions() {
			return queryOptions({
				queryKey: [{ namespace }, "actors", "count"] as QueryKey,
				enabled: true,
				queryFn: async () => {
					// TODO: fetch all actor names only to get the count is inefficient
					const namesList = await client.actorsListNames({
						namespace,
						limit: 100,
					});

					const names = Object.keys(namesList.names);

					const data = await Promise.all(
						names.map((name) =>
							client.actorsList({
								namespace,
								name,
								limit: 1,
								includeDestroyed: true,
							}),
						),
					);
					return data.reduce(
						(acc, curr) => acc + curr.actors.length,
						0,
					);
				},
				retry: shouldRetryAllExpect403,
				throwOnError: noThrow,
				meta: {
					mightRequireAuth,
				},
			});
		},
	};
};

class FetchError extends Error {
	constructor(
		message: string,
		public description: string,
	) {
		super(message);
	}
}

export function hasMetadataProvider(
	metadata: unknown,
): metadata is { provider?: string } {
	return z.object({ provider: z.string().optional() }).safeParse(metadata)
		.success;
}

export function hasProvider(
	configs:
		| [string, Rivet.RunnerConfigsListResponseRunnerConfigsValue][]
		| undefined,
	providers: string[],
): boolean {
	if (!configs) return false;
	return configs.some(([, config]) =>
		Object.values(config.datacenters).some(
			(datacenter) =>
				datacenter.metadata &&
				hasMetadataProvider(datacenter.metadata) &&
				datacenter.metadata.provider &&
				providers.includes(datacenter.metadata.provider),
		),
	);
}
