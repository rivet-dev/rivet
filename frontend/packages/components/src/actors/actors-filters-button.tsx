import { Button } from "@rivet-gg/components";
import { Icon, faFilter } from "@rivet-gg/icons";
import { ActorsFiltersSheet } from "./actors-filters-sheet";
import { useAtomValue, useSetAtom } from "jotai";
import { actorFiltersCountAtom, actorFiltersAtom } from "./actor-context";

export function ActorsFiltersButton() {
	const setFilters = useSetAtom(actorFiltersAtom);
	const filtersCount = useAtomValue(actorFiltersCountAtom);

	return (
		<ActorsFiltersSheet
			onFiltersSubmitted={(values) => {
				setFilters((old) => ({
					...old,
					showDestroyed: values.showDestroyed,
					tags: values.tags,
				}));
			}}
		>
			<Button
				size="sm"
				variant="ghost"
				startIcon={<Icon icon={faFilter} />}
			>
				Filters {filtersCount > 0 ? `(${filtersCount})` : ""}
			</Button>
		</ActorsFiltersSheet>
	);
}
