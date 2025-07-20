import { router } from "@/app";
import { queryClient } from "@/queries/global";
import type { Rivet } from "@rivet-gg/api-full";
import { type FilterValue, toRecord } from "@rivet-gg/components";
import {
	type Actor,
	type DestroyActor,
	type Logs,
	type Metrics,
	actorBuildsAtom,
	actorEnvironmentAtom,
	actorFiltersAtom,
	actorRegionsAtom,
	actorsAtom,
	actorsInternalFilterAtom,
	actorsPaginationAtom,
	actorsQueryAtom,
	createActorAtom,
	currentActorIdAtom,
	currentActorQueryAtom,
	getActorStatus,
} from "@rivet-gg/components/actors";
import {
	InfiniteQueryObserver,
	InfiniteQueryObserverSuccessResult,
	MutationObserver,
	QueryObserver,
} from "@tanstack/react-query";
import equal from "fast-deep-equal";
//import { createClient } from "actor-core/client";
import { type PrimitiveAtom, Provider, atom, createStore } from "jotai";
import { type ReactNode, useEffect, useState } from "react";
import {
	actorBuildsQueryOptions,
	actorLogsQueryOptions,
	actorMetricsQueryOptions,
	actorQueryOptions,
	actorRegionsQueryOptions,
	createActorEndpoint,
	destroyActorMutationOptions,
	projectActorsQueryOptions,
} from "../../queries";

interface ActorsProviderProps {
	actorId: string | undefined;
	projectNameId: string;
	environmentNameId: string;
	children?: ReactNode;
	fixedTags?: Record<string, string>;
	filter?: (actor: Rivet.actors.Actor) => boolean;
	isActorInternal?: (actor: Actor) => boolean;

	/// filters
	tags: FilterValue;
	region: FilterValue;
	createdAt: FilterValue;
	destroyedAt: FilterValue;
	status: FilterValue;
	devMode: FilterValue;
}

function mountActor({
	actor,
	projectNameId,
	environmentNameId,
}: {
	actor: Rivet.actors.Actor;
	projectNameId: string;
	environmentNameId: string;
}) {
	const destroy: PrimitiveAtom<DestroyActor> = atom({
		isDestroying: false as boolean,
		destroy: async () => {},
	});
	destroy.onMount = (set) => {
		const mutObserver = new MutationObserver(
			queryClient,
			destroyActorMutationOptions(),
		);

		set({
			destroy: async () => {
				await mutObserver.mutate({
					projectNameId,
					environmentNameId,
					actorId: actor.id,
				});
			},
			isDestroying: false,
		});

		mutObserver.subscribe((mutation) => {
			set({
				destroy: async () => {
					await mutation.mutate({
						projectNameId,
						environmentNameId,
						actorId: actor.id,
					});
				},
				isDestroying: mutation.isPending,
			});
		});

		return () => {
			mutObserver.reset();
		};
	};

	const logs = atom({
		logs: [] as Logs,
		status: "pending",
	});
	logs.onMount = (set) => {
		const logsObserver = new QueryObserver(
			queryClient,
			actorLogsQueryOptions({
				projectNameId,
				environmentNameId,
				actorId: actor.id,
			}),
		);

		type LogQuery = {
			status: string;
			data?: Awaited<
				ReturnType<
					Exclude<
						ReturnType<typeof actorLogsQueryOptions>["queryFn"],
						undefined
					>
				>
			>;
		};

		function updateStdOut(query: LogQuery) {
			const data = query.data;
			set((prev) => ({
				...prev,
				...data,
				status: query.status,
			}));
		}

		const subOut = logsObserver.subscribe((query) => {
			updateStdOut(query);
		});

		updateStdOut(logsObserver.getCurrentQuery().state);

		return () => {
			logsObserver.destroy();
			subOut();
		};
	};

	const metrics = atom({
		metrics: { cpu: null, memory: null } as Metrics,
		updatedAt: Date.now(),
		status: "pending",
	});
	metrics.onMount = (set) => {
		const metricsObserver = new QueryObserver(
			queryClient,
			actorMetricsQueryOptions(
				{
					projectNameId,
					environmentNameId,
					actorId: actor.id,
				},
				{ refetchInterval: 5000 },
			),
		);

		type MetricsQuery = {
			status: string;
			data?: Awaited<
				ReturnType<
					Exclude<
						ReturnType<typeof actorMetricsQueryOptions>["queryFn"],
						undefined
					>
				>
			>;
		};

		function updateMetrics(query: MetricsQuery) {
			const data = query.data;
			set((prev) => ({
				...prev,
				...data,
				status: query.status,
				updatedAt: Date.now(),
			}));
		}

		const subMetrics = metricsObserver.subscribe((query) => {
			updateMetrics(query);
		});

		updateMetrics(metricsObserver.getCurrentQuery().state);

		return () => {
			metricsObserver.destroy();
			subMetrics();
		};
	};

	return {
		...actor,
		logs,
		metrics,
		destroy,
		status: getActorStatus(actor),
	};
}

export function ActorsProvider({
	actorId,
	projectNameId,
	environmentNameId,
	children,
	fixedTags,
	filter,
	isActorInternal: internalFilter,
	// filters
	tags,
	region,
	createdAt,
	destroyedAt,
	status,
	devMode,
}: ActorsProviderProps) {
	const [store] = useState(() => createStore());

	// biome-ignore lint/correctness/useExhaustiveDependencies: store is not a dependency
	useEffect(() => {
		store.set(currentActorIdAtom, actorId);
		store.set(currentActorQueryAtom, { isLoading: false, error: null });
		if (!actorId) {
			return;
		}

		const actor = store.get(actorsAtom).find((a) => a.id === actorId);
		if (actor) {
			return;
		}

		store.set(currentActorQueryAtom, { isLoading: true, error: null });
		const observer = new QueryObserver(
			queryClient,
			actorQueryOptions({
				actorId,
				projectNameId,
				environmentNameId,
			}),
		);

		return observer.subscribe((query) => {
			store.set(currentActorQueryAtom, {
				isLoading: query.isLoading,
				error: query.error,
			});
			if (query.status === "success" && query.data) {
				store.set(actorsAtom, (actors) => {
					const existing = actors.find((a) => a.id === actorId);

					if (existing) {
						return actors.map((a) =>
							a.id === actorId
								? {
										...existing,
										...query.data,
										status: getActorStatus(query.data),
										endpoint: createActorEndpoint(
											query.data.network,
										),
										tags: toRecord(existing.tags),
									}
								: a,
						);
					}

					return [
						...actors,
						mountActor({
							actor: query.data,
							projectNameId,
							environmentNameId,
						}),
					];
				});
			}
		});
	}, [actorId]);

	// biome-ignore lint/correctness/useExhaustiveDependencies: store is not a dependency
	useEffect(() => {
		store.set(actorEnvironmentAtom, { projectNameId, environmentNameId });
	}, [projectNameId, environmentNameId]);

	// biome-ignore lint/correctness/useExhaustiveDependencies: store is not a dependency
	useEffect(() => {
		store.set(actorFiltersAtom, {
			tags,
			region,
			createdAt,
			destroyedAt,
			status,
			devMode,
		});
	}, [tags, region, createdAt, destroyedAt, status, devMode]);

	// biome-ignore lint/correctness/useExhaustiveDependencies:  store is not a dependency
	useEffect(() => {
		if (internalFilter) {
			store.set(actorsInternalFilterAtom, { fn: internalFilter });
		} else {
			store.set(actorsInternalFilterAtom, undefined);
		}
	}, [internalFilter]);

	// biome-ignore lint/correctness/useExhaustiveDependencies: store is not a dependency
	useEffect(() => {
		return store.sub(actorFiltersAtom, () => {
			const value = store.get(actorFiltersAtom);
			router.navigate({
				to: ".",
				search: (old) => ({
					...old,
					...value,
				}),
			});
		});
	}, [router]);

	// biome-ignore lint/correctness/useExhaustiveDependencies: store is not a dependency
	useEffect(() => {
		const actorsObserver = new InfiniteQueryObserver(
			queryClient,
			projectActorsQueryOptions({
				projectNameId,
				environmentNameId,
				includeDestroyed: true,
				tags: fixedTags,
			}),
		);

		const unsubFilters = store.sub(actorFiltersAtom, () => {
			actorsObserver.setOptions(
				projectActorsQueryOptions({
					projectNameId,
					environmentNameId,
					tags: fixedTags,
					includeDestroyed: true,
				}),
			);
			actorsObserver.refetch();
		});

		const unsub = actorsObserver.subscribe((query) => {
			store.set(actorsQueryAtom, {
				isLoading: query.isLoading,
				error: query.error?.message ?? null,
			});
			store.set(actorsPaginationAtom, {
				hasNextPage: query.hasNextPage,
				fetchNextPage: () => query.fetchNextPage(),
				isFetchingNextPage: query.isFetchingNextPage,
			});
			if (
				query.status === "success" &&
				query.data &&
				!query.isPlaceholderData
			) {
				store.set(actorsAtom, (actors) => {
					const additionalActors = upsertCurrentActor({
						store,
						query,
						actors,
						projectNameId,
						environmentNameId,
					});

					return [
						...additionalActors,
						...query.data
							.filter((actor) => filter?.(actor) ?? true)
							.map((actor) => {
								const existing = actors.find(
									(a) => a.id === actor.id,
								);
								if (existing) {
									return {
										...existing,
										...actor,
										status: getActorStatus(actor),
										endpoint: createActorEndpoint(
											actor.network,
										),
										tags: toRecord(existing.tags),
									};
								}

								return mountActor({
									actor,
									projectNameId,
									environmentNameId,
								});
							}),
					];
				});
			}
		});
		return () => {
			actorsObserver.destroy();
			unsub();
			unsubFilters();
		};
	}, [projectNameId, environmentNameId]);

	// biome-ignore lint/correctness/useExhaustiveDependencies: store is not a dependency
	useEffect(() => {
		const regionsObserver = new QueryObserver(
			queryClient,
			actorRegionsQueryOptions({ projectNameId, environmentNameId }),
		);

		const unsub = regionsObserver.subscribe((query) => {
			if (query.status === "success" && query.data) {
				store.set(actorRegionsAtom, query.data);
			}
		});

		return () => {
			regionsObserver.destroy();
			unsub();
		};
	}, [projectNameId, environmentNameId]);

	// biome-ignore lint/correctness/useExhaustiveDependencies: store is not a dependency
	useEffect(() => {
		const buildsObserver = new QueryObserver(
			queryClient,
			actorBuildsQueryOptions({
				projectNameId,
				environmentNameId,
			}),
		);
		const unsub = buildsObserver.subscribe((query) => {
			if (query.status === "success" && query.data) {
				store.set(actorBuildsAtom, (old) => {
					if (equal(old, query.data)) {
						return old;
					}
					return query.data;
				});
			}
		});
		return () => {
			buildsObserver.destroy();
			unsub();
		};
	}, [projectNameId, environmentNameId]);

	useEffect(() => {
		const mutationObserver = new MutationObserver(queryClient, {
			mutationFn: (data: {
				endpoint: string;
				id: string;
				tags: Record<string, string>;
				region?: string;
				params?: Record<string, unknown>;
			}) => {
				//const client = createClient(data.endpoint);
				//
				//const build = store
				//	.get(actorBuildsAtom)
				//	.find((build) => build.id === data.id);
				//
				//return client.create(build?.tags.name || "", {
				//	params: data.params,
				//	create: {
				//		tags: data.tags,
				//		region: data.region || undefined,
				//	},
				//});
			},
		});

		const storeSub = store.sub(actorsAtom, () => {
			const manager = store
				.get(actorsAtom)
				.find(
					(a) =>
						toRecord(a.tags).name === "manager" &&
						a.status === "running",
				);

			store.set(createActorAtom, (old) => {
				return {
					...old,
					endpoint: manager?.network
						? createActorEndpoint(manager.network) || null
						: null,
				};
			});
		});

		store.set(createActorAtom, (old) => ({
			...old,
			create: mutationObserver.mutate,
		}));

		const unsub = mutationObserver.subscribe((mutation) => {
			store.set(createActorAtom, (old) => ({
				...old,
				isCreating: mutation.isPending,
				create: mutation.mutate,
			}));
		});
		return () => {
			unsub();
			storeSub();
		};
	});

	return <Provider store={store}>{children}</Provider>;
}

function upsertCurrentActor({
	store,
	query,
	actors,
	projectNameId,
	environmentNameId,
}: {
	store: ReturnType<typeof createStore>;
	query: InfiniteQueryObserverSuccessResult<Rivet.actors.Actor[]>;
	actors: Actor[];
	projectNameId: string;
	environmentNameId: string;
	additionalActors?: Actor[];
}) {
	// check if current actor is in the query data
	const currentActorId = store.get(currentActorIdAtom);
	if (currentActorId) {
		// if there's no current actor in the query data, we upsert it from the query data
		const currentActor = query.data.find((a) => a.id === currentActorId);
		if (currentActor) {
			// it will be mounted in the next pass
			return [];
		}

		// check if it's not already in the actors list
		const existing = actors.find((a) => a.id === currentActorId);
		if (existing) {
			// if not, we add it to the additional actors
			return [existing];
		}

		// as a last resort, check the query data for the current actor
		const actorQuery = queryClient.getQueryData<Rivet.actors.Actor>(
			actorQueryOptions({
				actorId: currentActorId,
				projectNameId,
				environmentNameId,
			}).queryKey,
		);

		if (actorQuery) {
			return [
				mountActor({
					actor: actorQuery,
					projectNameId,
					environmentNameId,
				}),
			];
		}
	}
	return [];
}
