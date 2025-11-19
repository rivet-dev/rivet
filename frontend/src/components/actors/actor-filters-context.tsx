import { faHashtag, faKey } from "@rivet-gg/icons";
import { useSearch } from "@tanstack/react-router";
import { createContext, useContext } from "react";
import { useReadLocalStorage } from "usehooks-ts";
import { ls } from "../lib/utils";
import {
	createFiltersPicker,
	createFiltersRemover,
	createFiltersSchema,
	type FilterDefinitions,
	FilterOp,
	type FilterValue,
	type PickFiltersOptions,
} from "../ui/filters";

export const ACTORS_FILTERS_DEFINITIONS = {
	id: {
		type: "string",
		label: "Actor ID",
		icon: faHashtag,
		operators: [FilterOp.EQUAL],
		excludes: ["key"],
	},
	key: {
		type: "string",
		label: "Actor Key",
		icon: faKey,
		operators: [FilterOp.EQUAL],
		excludes: ["id"],
	},
	...(__APP_TYPE__ === "engine" || __APP_TYPE__ === "cloud"
		? {
				showDestroyed: {
					type: "boolean",
					label: "Show destroyed",
					category: "display",
				},
			}
		: {}),
	showIds: {
		type: "boolean",
		label: "Show IDs",
		category: "display",
		ephemeral: true,
	},
	...(__APP_TYPE__ === "engine" || __APP_TYPE__ === "cloud"
		? {
				showDatacenter: {
					type: "boolean",
					label: "Show Actors Datacenter",
					category: "display",
					ephemeral: true,
				},
			}
		: {}),
	wakeOnSelect: {
		type: "boolean",
		label: "Auto-wake Actors on select",
		category: "display",
		ephemeral: true,
		defaultValue: ["1"],
	},
} satisfies FilterDefinitions;

const defaultActorFiltersContextValue = {
	definitions: ACTORS_FILTERS_DEFINITIONS,
	get pick() {
		return createFiltersPicker(this.definitions);
	},
	get schema() {
		return createFiltersSchema(this.definitions);
	},
	get remove() {
		return createFiltersRemover(this.definitions);
	},
};

export const ActorsFilters = createContext(defaultActorFiltersContextValue);

export const ActorsFiltersProvider = ActorsFilters.Provider;

export const useActorsFilters = () => {
	const context = useContext(ActorsFilters);
	if (!context) {
		throw new Error("useActorsFilters must be used within ActorsFilters");
	}
	return context;
};

export function useFiltersValue(opts: PickFiltersOptions = {}) {
	const { pick } = useActorsFilters();
	const search = useSearch({
		from: "/_context",
		select: (state) => pick(state, { onlyStatic: true }),
	}) as Record<string, FilterValue | undefined>;

	const state = useReadLocalStorage(ls.actorsEphemeralFilters.key, {
		deserializer: (value) => JSON.parse(value),
		initializeWithValue: true,
	}) || { wakeOnSelect: { value: ["1"] } };

	if (opts.onlyEphemeral) {
		return state as Record<string, FilterValue | undefined>;
	}

	if (opts.onlyStatic) {
		return search as Record<string, FilterValue | undefined>;
	}

	return { ...search, ...state } as Record<string, FilterValue | undefined>;
}
