import { type Atom, atom } from "jotai";
import { atomFamily, splitAtom } from "jotai/utils";
import type { Rivet } from "@rivet-gg/api";
import { toRecord } from "../lib/utils";
import { ACTOR_FRAMEWORK_TAG_VALUE } from "./actor-tags";
import { FilterOp, type FilterValue } from "../ui/filters";
import { isAfter, isBefore } from "date-fns";

export enum ActorFeature {
	Logs = "logs",
	Config = "config",
	Connections = "connections",
	State = "state",
	Console = "console",
	Runtime = "runtime",
	InspectReconnectNotification = "inspect_reconnect_notification",
}

export type Actor = Omit<
	Rivet.actor.Actor,
	"createdAt" | "runtime" | "lifecycle" | "network" | "resources"
> & {
	status: "unknown" | "starting" | "running" | "stopped" | "crashed";

	lifecycle?: Rivet.actor.Lifecycle;
	endpoint?: string;
	logs: LogsAtom;
	network?: Rivet.actor.Network | null;
	resources?: Rivet.actor.Resources | null;
	runtime?: Rivet.actor.Runtime | null;
	destroy?: DestroyActorAtom;
	destroyTs?: Date;
	createdAt?: Date;
	features?: ActorFeature[];
};

export type Logs = {
	id: string;
	level: "error" | "info";
	timestamp: Date;
	line: string;
	message: string;
	properties: Record<string, unknown>;
}[];

export type Build = Rivet.actor.Build;
export type DestroyActor = {
	isDestroying: boolean;
	destroy: () => Promise<void>;
};

export type ActorAtom = Atom<Actor>;
export type LogsAtom = Atom<{
	logs: Logs;
	// query status
	status: string;
}>;
export type BuildAtom = Atom<Build>;
export type DestroyActorAtom = Atom<DestroyActor>;

export type CreateActor = {
	create: (values: {
		endpoint: string;
		id: string;
		tags: Record<string, string>;
		region?: string;
		params?: Record<string, unknown>;
	}) => Promise<unknown>;
	isCreating: boolean;
	endpoint: string | null;
};

export type Region = Rivet.actor.Region;

// global atoms
export const currentActorIdAtom = atom<string | undefined>(undefined);
export const actorsQueryAtom = atom<{
	isLoading: boolean;
	error: string | null;
}>({
	isLoading: false,
	error: null,
});
export const actorsAtom = atom<Actor[]>([]);
export const actorFiltersAtom = atom<{
	tags: FilterValue;
	region: FilterValue;
	createdAt: FilterValue;
	destroyedAt: FilterValue;
	status: FilterValue;
	devMode: FilterValue;
}>({
	tags: undefined,
	region: undefined,
	createdAt: undefined,
	destroyedAt: undefined,
	status: undefined,
	devMode: undefined,
});
export const actorsPaginationAtom = atom({
	hasNextPage: false,
	isFetchingNextPage: false,
	fetchNextPage: () => {},
});

export const actorRegionsAtom = atom<Region[]>([
	{
		id: "local",
		name: "Local",
	},
]);

export const actorBuildsAtom = atom<Build[]>([]);

export const actorsInternalFilterAtom = atom<{
	fn: (actor: Actor) => boolean;
}>();

// derived atoms

export const currentActorRegionAtom = atom((get) => {
	const actorAtom = get(currentActorAtom);
	if (!actorAtom) {
		return undefined;
	}
	const regions = get(actorRegionsAtom);
	const actor = get(actorAtom);
	return regions.find((region) => region.id === actor.region);
});
export const filteredActorsAtom = atom((get) => {
	const filters = get(actorFiltersAtom);
	const actors = get(actorsAtom);

	const isActorInternal = get(actorsInternalFilterAtom)?.fn;

	return actors.filter((actor) => {
		const satisfiesFilters = Object.entries(filters).every(
			([key, filter]) => {
				if (filter === undefined) {
					return true;
				}
				if (key === "tags") {
					const filterTags = filter.value.map((tag) =>
						tag.split("="),
					);
					const tags = toRecord(actor.tags);

					if (filter.operator === FilterOp.NOT_EQUAL) {
						return Object.entries(tags).every(
							([tagKey, tagValue]) => {
								return filterTags.every(
									([filterKey, filterValue]) => {
										if (filterKey === tagKey) {
											if (filterValue === "*") {
												return false;
											}
											return tagValue !== filterValue;
										}
										return true;
									},
								);
							},
						);
					}

					return Object.entries(tags).some(([tagKey, tagValue]) => {
						return filterTags.some(([filterKey, filterValue]) => {
							if (filterKey === tagKey) {
								if (filterValue === "*") {
									return true;
								}
								return tagValue === filterValue;
							}
							return false;
						});
					});
				}

				if (key === "region") {
					if (filter.operator === FilterOp.NOT_EQUAL) {
						return !filter.value.includes(actor.region);
					}

					return filter.value.includes(actor.region);
				}

				if (key === "createdAt") {
					if (actor.createdAt === undefined) {
						return false;
					}
					const createdAt = new Date(actor.createdAt);

					if (filter.operator === FilterOp.AFTER) {
						return isAfter(createdAt, +filter.value[0]);
					}
					if (filter.operator === FilterOp.BEFORE) {
						return isBefore(createdAt, +filter.value[0]);
					}
					if (filter.operator === FilterOp.BETWEEN) {
						return (
							isAfter(createdAt, +filter.value[0]) &&
							isBefore(createdAt, +filter.value[1])
						);
					}
					return false;
				}

				if (key === "destroyedAt") {
					if (actor.destroyTs === undefined) {
						return false;
					}
					const destroyedAt = new Date(actor.destroyTs);

					if (filter.operator === FilterOp.AFTER) {
						return isAfter(destroyedAt, +filter.value[0]);
					}
					if (filter.operator === FilterOp.BEFORE) {
						return isBefore(destroyedAt, +filter.value[0]);
					}
					if (filter.operator === FilterOp.BETWEEN) {
						return (
							isAfter(destroyedAt, +filter.value[0]) &&
							isBefore(destroyedAt, +filter.value[1])
						);
					}
					return false;
				}

				if (key === "status") {
					if (filter.operator === FilterOp.NOT_EQUAL) {
						return !filter.value.includes(actor.status);
					}

					return filter.value.includes(actor.status);
				}

				return true;
			},
		);

		const isInternal =
			toRecord(actor.tags).owner === "rivet" ||
			(isActorInternal?.(actor) ?? false);

		return (
			satisfiesFilters && ((isInternal && filters.devMode) || !isInternal)
		);
	});
});
export const actorsAtomsAtom = splitAtom(
	filteredActorsAtom,
	(actor) => actor.id,
);
export const actorsCountAtom = atom((get) => get(actorsAtom).length);
export const filteredActorsCountAtom = atom(
	(get) => get(filteredActorsAtom).length,
);

export const currentActorAtom = atom((get) => {
	const actorId = get(currentActorIdAtom);
	return get(actorsAtomsAtom).find((actor) => get(actor).id === actorId);
});

export const isCurrentActorAtom = atomFamily((actor: ActorAtom) =>
	atom((get) => {
		const actorId = get(currentActorIdAtom);
		return get(actor).id === actorId;
	}),
);

export const actorFiltersCountAtom = atom((get) => {
	const filters = get(actorFiltersAtom);
	return Object.values(filters).filter((value) => value !== undefined).length;
});

// tags created by the user, not from the server
export const actorCustomTagValues = atom<string[]>([]);
export const actorCustomTagKeys = atom<string[]>([]);

const actorCustomTagsAtom = atom<{ keys: string[]; values: string[] }>(
	(get) => {
		const keys = get(actorCustomTagKeys);
		const values = get(actorCustomTagValues);

		return { keys, values };
	},
	// @ts-expect-error
	(get, set, value: { key: string; value: string }) => {
		set(actorCustomTagKeys, (keys) => {
			const newKeys = [...keys];
			const index = newKeys.indexOf(value.key);
			if (index === -1) {
				newKeys.push(value.key);
			}
			return newKeys;
		});
		set(actorCustomTagValues, (values) => {
			const newValues = [...values];
			const index = newValues.indexOf(value.value);
			if (index === -1) {
				newValues.push(value.value);
			}
			return newValues;
		});
	},
);

export const createActorAtom = atom<CreateActor>({
	endpoint: null,
	isCreating: false,
	create: async () => {},
});

export const actorManagerEndpointAtom = atom<string | null>((get) => {
	return get(createActorAtom)?.endpoint ?? null;
});

export const actorTagsAtom = atom((get) => {
	const actorTags = get(actorsAtom).flatMap((actor) =>
		Object.entries(toRecord(actor.tags)).map(([key, value]) => ({
			key,
			value: value as string,
		})),
	);

	const keys = new Set<string>();
	const values = new Set<string>();

	for (const { key, value } of actorTags) {
		keys.add(key);
		values.add(value);
	}

	const customTags = get(actorCustomTagsAtom);

	for (const key of customTags.keys) {
		keys.add(key);
	}

	for (const value of customTags.values) {
		values.add(value);
	}

	const allTags = [];

	for (const key of keys) {
		for (const value of values) {
			allTags.push({ key, value });
		}
	}

	return allTags;
});

export const actorTagValuesAtom = atom((get) => {
	const tags = get(actorTagsAtom);
	const values = new Set<string>();
	for (const tag of tags) {
		values.add(tag.value);
	}
	return [...values];
});

export const actorTagKeysAtom = atom((get) => {
	const tags = get(actorTagsAtom);
	const keys = new Set<string>();
	for (const tag of tags) {
		keys.add(tag.key);
	}
	return [...keys];
});

export const actorBuildsCountAtom = atom((get) => {
	return get(actorBuildsAtom).length;
});

const commonActorFeatures = [
	ActorFeature.Logs,
	ActorFeature.Config,
	ActorFeature.Runtime,
	ActorFeature.InspectReconnectNotification,
];

export const currentActorFeaturesAtom = atom((get) => {
	const atom = get(currentActorAtom);
	if (!atom) {
		return [];
	}

	const actor = get(atom);

	// actors from hub
	if (!actor.features) {
		const tags = toRecord(actor.tags);
		if (tags.framework === ACTOR_FRAMEWORK_TAG_VALUE) {
			if (tags.name === "manager") {
				return commonActorFeatures;
			}
			return [
				...commonActorFeatures,
				ActorFeature.Connections,
				ActorFeature.State,
				ActorFeature.Console,
				ActorFeature.InspectReconnectNotification,
			];
		}
		return commonActorFeatures;
	}

	return actor.features;
});
