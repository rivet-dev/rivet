import { Icon, faQuestionSquare } from "@rivet-gg/icons";
import { useActorsView } from "./actors-view-context-provider";
import { actorFiltersAtom, type ActorFeature } from "./actor-context";
import { ActorTabs } from "./actors-actor-details";
import { useAtomValue, useSetAtom } from "jotai";
import { useCallback } from "react";
import { selectAtom } from "jotai/utils";
import { Button } from "../ui/button";
import { FilterOp } from "../ui/filters";

export function ActorNotFound({
	features = [],
}: { features?: ActorFeature[] }) {
	const { copy } = useActorsView();

	const setFilters = useSetAtom(actorFiltersAtom);
	const hasDevMode = useAtomValue(
		selectAtom(
			actorFiltersAtom,
			useCallback((filters) => filters.devMode, []),
		),
	);

	return (
		<div className="flex flex-col h-full flex-1 pt-2">
			<ActorTabs disabled features={features}>
				<div className="flex text-center text-foreground flex-1 justify-center items-center flex-col gap-2">
					<Icon icon={faQuestionSquare} className="text-4xl" />
					<p className="max-w-[400px]">{copy.actorNotFound}</p>
					<p className="max-w-[400px] text-sm text-muted-foreground">
						{copy.actorNotFoundDescription}
					</p>

					{!hasDevMode ? (
						<Button
							className="mt-3"
							variant="outline"
							size="sm"
							onClick={() => {
								setFilters((prev) => ({
									...prev,
									devMode: {
										value: ["true"],
										operator: FilterOp.EQUAL,
									},
								}));
							}}
						>
							{copy.showHiddenActors}
						</Button>
					) : null}
				</div>
			</ActorTabs>
		</div>
	);
}
