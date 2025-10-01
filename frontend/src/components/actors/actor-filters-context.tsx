import { faHashtag, faKey } from "@rivet-gg/icons";
import { useSearch } from "@tanstack/react-router";
import { createContext, useContext } from "react";
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
	...(__APP_TYPE__ === "engine"
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

export const useFilters = (
	fn: (filters: Record<string, any>) => any = (state) => state,
): any => {
	const { pick } = useActorsFilters();
	return useSearch({
		strict: false,
		select: (state) => fn(pick(state)),
	}) as Record<string, FilterValue | undefined>;
};

export function useFiltersValue(opts: PickFiltersOptions = {}) {
	const { pick } = useActorsFilters();
	return useSearch({
		from: "/_context",
		select: (state) => pick(state, opts),
	}) as Record<string, FilterValue | undefined>;
}
