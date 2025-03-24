import { Button } from "@rivet-gg/components";
import { Icon, faFilter } from "@rivet-gg/icons";
import { ActorsFiltersSheet } from "./actors-filters-sheet";

import { Route as ActorsRoute } from "@/routes/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/actors";
import { useEnvironment } from "../../data/environment-context";
import { useProject } from "../../data/project-context";

interface ActorsFiltersButtonProps {
	showDestroyed: boolean;
	tags: Record<string, string>;
}

export function ActorsFiltersButton({
	showDestroyed,
	tags,
}: ActorsFiltersButtonProps) {
	const { gameId: projectId } = useProject();
	const { namespaceId: environmentId } = useEnvironment();

	const navigate = ActorsRoute.useNavigate();

	const hasFilters = Object.keys(tags).length > 0 || !showDestroyed;

	return (
		<ActorsFiltersSheet
			title="Filters"
			projectId={projectId}
			environmentId={environmentId}
			tags={tags}
			showDestroyed={showDestroyed ?? true}
			onFiltersSubmitted={(values) => {
				return navigate({
					search: (old) => ({
						...old,
						showDestroyed: values.showDestroyed,
						tags: Object.entries(values.tags).map(
							([key, value]) => [key, value] as [string, string],
						),
					}),
				});
			}}
		>
			<Button
				size="sm"
				variant="ghost"
				startIcon={<Icon icon={faFilter} />}
			>
				Filters{" "}
				{hasFilters
					? `(${Object.keys(tags).length + (+!showDestroyed || 0)})`
					: ""}
			</Button>
		</ActorsFiltersSheet>
	);
}
