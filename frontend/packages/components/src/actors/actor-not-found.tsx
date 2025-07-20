import { Icon, faQuestionSquare } from "@rivet-gg/icons";
import { useAtomValue, useSetAtom } from "jotai";
import { selectAtom } from "jotai/utils";
import { useCallback } from "react";
import { Button } from "../ui/button";
import { FilterOp } from "../ui/filters";
import {
	type ActorFeature,
	actorFiltersAtom,
	currentActorQueryAtom,
} from "./actor-context";
import { ActorTabs } from "./actors-actor-details";
import { useActorsView } from "./actors-view-context-provider";
import { ShimmerLine } from "../shimmer-line";

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

	const { isLoading } = useAtomValue(currentActorQueryAtom);

	return (
		<div className="flex flex-col h-full flex-1">
			<ActorTabs disabled features={features} className="relative">
				<div className="flex text-center text-foreground flex-1 justify-center items-center flex-col relative gap-2">
					{!isLoading ? (
						<>
							<Icon
								icon={faQuestionSquare}
								className="text-4xl"
							/>
							<p className="max-w-[400px]">
								{copy.actorNotFound}
							</p>
							<p className="max-w-[400px] text-sm text-muted-foreground">
								{copy.actorNotFoundDescription}
							</p>
						</>
					) : null}

					{!hasDevMode && !isLoading ? (
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
					{isLoading ? <ShimmerLine className="top-0" /> : null}
				</div>
			</ActorTabs>
		</div>
	);
}
