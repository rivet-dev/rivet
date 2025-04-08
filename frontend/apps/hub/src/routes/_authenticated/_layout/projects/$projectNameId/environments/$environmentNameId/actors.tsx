import {
	actorBuildsAtom,
	actorFiltersAtom,
	actorRegionsAtom,
	ActorsActorDetails,
	ActorsActorDetailsPanel,
	actorsAtom,
	ActorsListPreview,
	actorsPaginationAtom,
	createActorAtom,
	currentActorAtom,
	currentActorIdAtom,
	type DestroyActor,
	getActorStatus,
} from "@rivet-gg/components/actors";
import { createClient } from "actor-core/client";
import { useEnvironment } from "@/domains/project/data/environment-context";
import { useProject } from "@/domains/project/data/project-context";
import * as Layout from "@/domains/project/layouts/servers-layout";
import {
	actorBuildsCountQueryOptions,
	actorLogsQueryOptions,
	actorRegionsQueryOptions,
	createActorEndpoint,
	projectActorsQueryOptions,
	destroyActorMutationOptions,
	actorBuildsQueryOptions,
} from "@/domains/project/queries";
import {
	InfiniteQueryObserver,
	QueryObserver,
	MutationObserver,
	useSuspenseQuery,
} from "@tanstack/react-query";
import { createFileRoute, useRouter } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { z } from "zod";
import { GettingStarted } from "@rivet-gg/components/actors";
import {
	atom,
	createStore,
	type PrimitiveAtom,
	Provider,
	useAtomValue,
} from "jotai";
import { useEffect } from "react";
import { queryClient } from "@/queries/global";
import { toRecord } from "@rivet-gg/components";
import { useDialog } from "@/hooks/use-dialog";
import equal from "fast-deep-equal";

function Actor() {
	const navigate = Route.useNavigate();
	const { tab } = Route.useSearch();

	const actor = useAtomValue(currentActorAtom);

	if (!actor) {
		return null;
	}

	return (
		<ActorsActorDetails
			actor={actor}
			tab={tab}
			onTabChange={(tab) => {
				navigate({
					to: ".",
					search: (old) => ({ ...old, tab }),
				});
			}}
		/>
	);
}

const store = createStore();

function Content() {
	const { nameId: projectNameId } = useProject();
	const { nameId: environmentNameId } = useEnvironment();
	const { actorId, tags, showDestroyed, modal } = Route.useSearch();

	const CreateActorDialog = useDialog.CreateActor.Dialog;
	const GoToActorDialog = useDialog.GoToActor.Dialog;
	const router = useRouter();
	const navigate = Route.useNavigate();

	useEffect(() => {
		store.set(currentActorIdAtom, actorId);
	}, [actorId]);

	useEffect(() => {
		store.set(actorFiltersAtom, {
			showDestroyed: showDestroyed ?? true,
			tags: Object.fromEntries(
				tags?.map((tag) => [tag[0], tag[1]]) || [],
			),
		});

		store.set(currentActorIdAtom, actorId);
	}, [tags, showDestroyed, actorId]);

	useEffect(() => {
		return store.sub(actorFiltersAtom, () => {
			const value = store.get(actorFiltersAtom);
			router.navigate({
				to: ".",
				search: (old) => ({
					...old,
					tags: Object.entries(value.tags).map(([key, value]) => [
						key,
						value,
					]),
					showDestroyed: value.showDestroyed,
				}),
			});
		});
	}, [router]);

	useEffect(() => {
		const defaultFilters = store.get(actorFiltersAtom);
		const actorsObserver = new InfiniteQueryObserver(
			queryClient,
			projectActorsQueryOptions({
				projectNameId,
				environmentNameId,
				includeDestroyed: defaultFilters.showDestroyed,
				tags: defaultFilters.tags,
			}),
		);

		const unsubFilters = store.sub(actorFiltersAtom, () => {
			const filters = store.get(actorFiltersAtom);
			actorsObserver.setOptions(
				projectActorsQueryOptions({
					projectNameId,
					environmentNameId,
					tags: filters.tags,
					includeDestroyed: filters.showDestroyed,
				}),
			);
			actorsObserver.refetch();
		});

		const unsub = actorsObserver.subscribe((query) => {
			store.set(actorsPaginationAtom, {
				hasNextPage: query.hasNextPage,
				fetchNextPage: () => query.fetchNextPage(),
				isFetchingNextPage: query.isFetchingNextPage,
			});
			if (query.status === "success" && query.data) {
				store.set(actorsAtom, (actors) => {
					return query.data.map((actor) => {
						const existing = actors.find((a) => a.id === actor.id);
						if (existing) {
							return {
								...existing,
								...actor,
								status: getActorStatus(actor),
								endpoint: createActorEndpoint(actor.network),
								tags: {
									...toRecord(existing.tags),
									framework: "actor-core",
								},
							};
						}

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
							logs: { lines: [], timestamps: [], ids: [] },
							errors: { lines: [], timestamps: [], ids: [] },
						});
						logs.onMount = (set) => {
							const stdOutObserver = new QueryObserver(
								queryClient,
								actorLogsQueryOptions({
									projectNameId,
									environmentNameId,
									actorId: actor.id,
									stream: "std_out",
								}),
							);
							const stdErrObserver = new QueryObserver(
								queryClient,
								actorLogsQueryOptions({
									projectNameId,
									environmentNameId,
									actorId: actor.id,
									stream: "std_err",
								}),
							);

							function updateStdOut(query: any) {
								if (query.status === "success" && query.data) {
									set((prev) => ({
										...prev,
										logs: {
											lines: query.data.lines,
											timestamps: query.data.timestamps,
											ids: query.data.ids,
										},
									}));
								}
							}

							function updateStdErr(query: any) {
								if (query.status === "success" && query.data) {
									set((prev) => ({
										...prev,
										errors: {
											lines: query.data.lines,
											timestamps: query.data.timestamps,
											ids: query.data.ids,
										},
									}));
								}
							}

							const subOut = stdOutObserver.subscribe((query) => {
								updateStdOut(query);
							});

							const subErr = stdErrObserver.subscribe((query) => {
								updateStdErr(query);
							});

							updateStdOut(stdOutObserver.getCurrentQuery());
							updateStdErr(stdErrObserver.getCurrentQuery());

							return () => {
								stdOutObserver.destroy();
								stdErrObserver.destroy();
								subOut();
								subErr();
							};
						};

						return {
							...actor,
							logs,
							destroy,
							status: getActorStatus(actor),
						};
					});
				});
			}
		});
		return () => {
			actorsObserver.destroy();
			unsub();
			unsubFilters();
		};
	}, [projectNameId, environmentNameId]);

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
				const client = createClient(data.endpoint);

				const build = store
					.get(actorBuildsAtom)
					.find((build) => build.id === data.id);

				return client.create(build?.tags.name || "", {
					params: data.params,
					create: {
						tags: data.tags,
						region: data.region || undefined,
					},
				});
			},
		});

		const storeSub = store.sub(actorsAtom, () => {
			const manager = store
				.get(actorsAtom)
				.find(
					(a) =>
						toRecord(a.tags).name === "manager" &&
						toRecord(a.tags).owner === "rivet" &&
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

	function handleOpenChange(open: boolean) {
		router.navigate({
			to: ".",
			search: (old) => ({
				...old,
				modal: !open ? undefined : modal,
			}),
		});
	}

	return (
		<Provider store={store}>
			<ActorsListPreview>
				<ActorsActorDetailsPanel actorId={actorId}>
					{actorId ? <Actor /> : null}
				</ActorsActorDetailsPanel>
			</ActorsListPreview>

			<CreateActorDialog
				dialogProps={{
					open: modal === "create-actor",
					onOpenChange: handleOpenChange,
				}}
			/>
			<GoToActorDialog
				onSubmit={(actorId) => {
					navigate({
						to: ".",
						search: (old) => ({
							...old,
							actorId,
							modal: undefined,
						}),
					});
				}}
				dialogProps={{
					open: modal === "go-to-actor",
					onOpenChange: handleOpenChange,
				}}
			/>
		</Provider>
	);
}

function ProjectActorsRoute() {
	const { nameId: projectNameId } = useProject();
	const { nameId: environmentNameId } = useEnvironment();
	const { tags, showDestroyed } = Route.useSearch();

	const { data } = useSuspenseQuery({
		...actorBuildsCountQueryOptions({
			projectNameId,
			environmentNameId,
		}),
		refetchInterval: (query) =>
			(query.state.data?.builds.length || 0) > 0 ? false : 2000,
	});

	if (data === 0 && !tags && showDestroyed === undefined) {
		return <GettingStarted />;
	}

	return (
		<div className="flex flex-col w-screen h-[calc(100vh-6.5rem)] bg-card -mx-4 -my-4">
			<Content />
		</div>
	);
}

const searchSchema = z.object({
	actorId: z.string().optional(),
	tab: z.string().optional(),

	tags: z.array(z.tuple([z.string(), z.string()])).optional(),
	showDestroyed: z.boolean().optional().default(true),
});

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/actors",
)({
	validateSearch: zodValidator(searchSchema),
	staticData: {
		layout: "actors",
	},
	component: ProjectActorsRoute,
	pendingComponent: Layout.Content.Skeleton,
});
