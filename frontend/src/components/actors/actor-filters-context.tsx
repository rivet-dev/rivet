import { faHashtag, faKey } from "@rivet-gg/icons";
import { useSearch } from "@tanstack/react-router";
import { createContext, useContext } from "react";
import {
	createFiltersPicker,
	createFiltersRemover,
	createFiltersSchema,
	type FilterDefinitions,
	FilterOp,
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
	// tags: {
	// 	type: "select",
	// 	label: "Tags",
	// 	icon: faTag,
	// 	options: TagsOptions,
	// 	operators: {
	// 		[FilterOp.EQUAL]: "is one of",
	// 		[FilterOp.NOT_EQUAL]: "is not one of",
	// 	},
	// },
	// createdAt: {
	// 	type: "date",
	// 	label: "Created",
	// 	icon: faCalendarCirclePlus,
	// },
	// destroyedAt: {
	// 	type: "date",
	// 	label: "Destroyed",
	// 	icon: faCalendarCircleMinus,
	// },
	// status: {
	// 	type: "select",
	// 	label: "Status",
	// 	icon: faSignalBars,
	// 	options: StatusOptions,
	// 	display: ({ value }) => {
	// 		if (value.length > 1) {
	// 			return <span>{value.length} statuses</span>;
	// 		}
	// 		return (
	// 			<ActorStatus
	// 				className="border-0 p-0"
	// 				status={value[0] as ActorStatusType}
	// 			/>
	// 		);
	// 	},
	// },
	// region: {
	// 	type: "select",
	// 	label: "Region",
	// 	icon: faGlobe,
	// 	options: RegionOptions,
	// 	display: ({ value }) => {
	// 		if (value.length > 1) {
	// 			return <span>{value.length} regions</span>;
	// 		}

	// 		return <ActorRegion regionId={value[0]} showLabel />;
	// 	},
	// 	operators: {
	// 		[FilterOp.EQUAL]: "is one of",
	// 		[FilterOp.NOT_EQUAL]: "is not one of",
	// 	},
	// },
	// destroyed: {
	// 	type: "boolean",
	// 	label: "Show destroyed actors",
	// 	icon: faEye,
	// },
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
	});
};

export function useFiltersValue(opts: PickFiltersOptions = {}) {
	const { pick } = useActorsFilters();
	return useSearch({
		from: "/_layout",
		select: (state) => pick(state, opts),
	});
}
