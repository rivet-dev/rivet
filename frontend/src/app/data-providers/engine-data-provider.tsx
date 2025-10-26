import { type Rivet, RivetClient } from "@rivetkit/engine-api-full";
import { fetcher } from "@rivetkit/engine-api-full/core";
import {
	infiniteQueryOptions,
	mutationOptions,
	type QueryKey,
	queryOptions,
	skipToken,
} from "@tanstack/react-query";
import z from "zod";
import { getConfig, ls } from "@/components";
import {
	type Actor,
	ActorFeature,
	type ActorId,
	type CrashPolicy,
} from "@/components/actors";
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
			return await fetcher(args);
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
				queryFn: async ({ pageParam, signal: abortSignal }) => {
					const data = await client.namespaces.list(
						{
							limit: RECORDS_PER_PAGE,
							cursor: pageParam ?? undefined,
						},
						{ abortSignal },
					);
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
		statusQueryOptions() {
			return queryOptions({
				...def.statusQueryOptions(),
				queryKey: [{ namespace }, ...def.statusQueryOptions().queryKey],
				enabled: true,
				queryFn: async () => {
					return true;
				},
				retry: shouldRetryAllExpect403,
				throwOnError: noThrow,
				meta: {
					mightRequireAuth,
				},
			});
		},
		regionsQueryOptions() {
			return infiniteQueryOptions({
				...def.regionsQueryOptions(),
				enabled: true,
				queryKey: [
					{ namespace },
					...def.regionsQueryOptions().queryKey,
				] as QueryKey,
				queryFn: async () => {
					const data = await client.datacenters.list();
					return {
						regions: data.datacenters.map((dc) => ({
							id: dc.name,
							name: dc.name,
							url: dc.url,
						})),
						pagination: data.pagination,
					};
				},
				retry: shouldRetryAllExpect403,
				throwOnError: noThrow,
				meta: {
					mightRequireAuth,
				},
			});
		},
		regionQueryOptions(regionId: string | undefined) {
			return queryOptions({
				...def.regionQueryOptions(regionId),
				queryKey: [
					{ namespace },
					...def.regionQueryOptions(regionId).queryKey,
				],
				queryFn: async ({ client }) => {
					const regions = await client.ensureInfiniteQueryData(
						this.regionsQueryOptions(),
					);

					for (const page of regions.pages) {
						for (const region of page.regions) {
							if (region.id === regionId) {
								return region;
							}
						}
					}

					throw new Error(`Region not found: ${regionId}`);
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
				queryFn: async ({ signal: abortSignal }) => {
					const data = await client.actorsList(
						{ actorIds: actorId as string, namespace },
						{ abortSignal },
					);

					if (!data.actors[0]) {
						throw new Error("Actor not found");
					}

					return transformActor(data.actors[0]);
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
				queryFn: async ({
					signal: abortSignal,
					pageParam,
					queryKey: [, , _opts],
				}) => {
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

					const data = await client.actorsList(
						{
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
							limit: RECORDS_PER_PAGE,
							name: opts?.filters?.id?.value
								? undefined
								: opts?.n?.join(","),
						},
						{ abortSignal },
					);

					return {
						...data,
						actors: data.actors.map((actor) =>
							transformActor(actor),
						),
					};
				},
				getNextPageParam: (lastPage) => {
					if (lastPage.actors.length < RECORDS_PER_PAGE) {
						return undefined;
					}
					return lastPage.pagination.cursor;
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
				queryFn: async ({ signal: abortSignal, pageParam }) => {
					const data = await client.actorsListNames(
						{
							namespace,
							cursor: pageParam ?? undefined,
							limit: RECORDS_PER_PAGE,
						},
						{ abortSignal },
					);

					return {
						pagination: data.pagination,
						builds: Object.keys(data.names)
							.sort()
							.map((build) => ({
								id: build,
								name: build,
							})),
					};
				},
				getNextPageParam: (lastPage) => {
					if (lastPage.builds.length < RECORDS_PER_PAGE) {
						return undefined;
					}
					return lastPage.pagination.cursor;
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
				mutationKey: [namespace, "actors"],
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
				queryFn: async ({ signal: abortSignal }) => {
					const res = await client.runnerConfigsServerlessHealthCheck(
						{
							url: opts.runnerUrl,
							headers: opts.headers,
							namespace,
						},
						{ abortSignal },
					);

					if ("success" in res) {
						return res.success;
					}

					throw res.failure;
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
				queryFn: async ({ pageParam, signal: abortSignal }) => {
					const data = await client.runners.list(
						{
							namespace,
							cursor: pageParam ?? undefined,
							limit: RECORDS_PER_PAGE,
						},
						{ abortSignal },
					);
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
				queryFn: async ({ signal: abortSignal, pageParam }) => {
					const data = await client.runners.listNames(
						{
							namespace,
							cursor: pageParam ?? undefined,
							limit: RECORDS_PER_PAGE,
						},
						{
							abortSignal,
						},
					);
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
				queryFn: async ({ signal: abortSignal }) => {
					const data = await client.runners.list(
						{
							namespace: opts.namespace,
							runnerIds: opts.runnerId,
						},
						{
							abortSignal,
						},
					);

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
				queryFn: async ({ signal: abortSignal }) => {
					const data = await client.runners.list(
						{ namespace, name: opts.runnerName },
						{
							abortSignal,
						},
					);
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
			opts: { onSuccess?: () => void } = {},
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
				queryFn: async ({ signal: abortSignal, pageParam }) => {
					const response = await client.runnerConfigsList(
						{
							namespace,
							cursor: pageParam ?? undefined,
							limit: RECORDS_PER_PAGE,
							variant: opts?.variant,
						},
						{ abortSignal },
					);

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
				queryFn: async ({ signal: abortSignal }) => {
					const response = await client.runnerConfigsList(
						{
							namespace,
							runnerNames: opts.name,
							variant: opts.variant,
						},
						{ abortSignal },
					);

					// biome-ignore lint/style/noNonNullAssertion: at this point, we know name is defined
					const config = response.runnerConfigs[opts.name!];

					if (!config) {
						throw new Error("Runner config not found");
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
	};
};

function transformActor(a: Rivet.Actor): Actor {
	return {
		id: a.actorId as ActorId,
		name: a.name,
		key: a.key ? a.key : undefined,
		connectableAt: a.connectableTs
			? new Date(a.connectableTs).toISOString()
			: undefined,
		region: a.datacenter,
		createdAt: new Date(a.createTs).toISOString(),
		startedAt: a.startTs ? new Date(a.startTs).toISOString() : undefined,
		destroyedAt: a.destroyTs
			? new Date(a.destroyTs).toISOString()
			: undefined,
		sleepingAt: a.sleepTs ? new Date(a.sleepTs).toISOString() : undefined,
		pendingAllocationAt: a.pendingAllocationTs
			? new Date(a.pendingAllocationTs).toISOString()
			: undefined,
		crashPolicy: a.crashPolicy as CrashPolicy,
		runner: a.runnerNameSelector,
		features: [
			ActorFeature.Config,
			ActorFeature.Connections,
			ActorFeature.State,
			ActorFeature.Console,
			ActorFeature.Database,
			ActorFeature.EventsMonitoring,
		],
	};
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
