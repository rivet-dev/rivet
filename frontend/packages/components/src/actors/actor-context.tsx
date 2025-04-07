import { type Atom, atom } from "jotai";
import { atomFamily, splitAtom } from "jotai/utils";
import type { Rivet } from "@rivet-gg/api";
import { toRecord } from "../lib/utils";
import { ACTOR_FRAMEWORK_TAG_VALUE } from "./actor-tags";

export enum ActorFeature {
	Logs = "logs",
	Config = "config",
	Connections = "connections",
	State = "state",
	Console = "console",
	Runtime = "runtime",
	Durability = "durability",
	InspectReconnectNotification = "inspect_reconnect_notification",
}

export type Actor = Omit<
	Rivet.actor.Actor,
	"createdAt" | "runtime" | "lifecycle" | "network" | "resources"
> & {
	status: "unknown" | "starting" | "running" | "stopped" | "crashed";

	lifecycle?: Rivet.actor.Lifecycle;
	build?: Rivet.actor.Build;
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
	lines: string[];
	timestamps: string[];
	ids: string[];
};

export type Build = Rivet.actor.Build;
export type DestroyActor = {
	isDestroying: boolean;
	destroy: () => Promise<void>;
};

export type ActorAtom = Atom<Actor>;
export type LogsAtom = Atom<{
	logs: Logs;
	errors: Logs;
}>;
export type BuildAtom = Atom<Build>;
export type DestroyActorAtom = Atom<DestroyActor>;

export type CreateActor = {
	create: (values: {
		endpoint: string;
		name: string;
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
export const actorsAtom = atom<Actor[]>([]);
export const actorFiltersAtom = atom<{
	showDestroyed: boolean;
	tags: Record<string, string>;
}>({
	showDestroyed: true,
	tags: {},
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
	const { showDestroyed, tags } = get(actorFiltersAtom);
	const actors = get(actorsAtom);

	return actors.filter((actor) => {
		if (!showDestroyed && actor.destroyTs) {
			return false;
		}
		if (Object.keys(tags).length === 0) {
			return true;
		}
		for (const [key, value] of Object.entries(tags)) {
			if (toRecord(actor.tags)[key] !== value) {
				return false;
			}
		}
		return true;
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
	const { showDestroyed, tags } = get(actorFiltersAtom);
	return Object.keys(tags).length + (+!showDestroyed || 0);
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
	const actorTags = get(actorsAtomsAtom).flatMap((actor) =>
		Object.entries(toRecord(get(actor).tags)).map(([key, value]) => ({
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
	ActorFeature.Durability,
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
		if (toRecord(actor.tags).framework === ACTOR_FRAMEWORK_TAG_VALUE) {
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
