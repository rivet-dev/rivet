import { faQuestionSquare, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { Button } from "../ui/button";
import { FilterOp } from "../ui/filters";
import { ActorDetailsSkeleton } from "./actor-details-skeleton";
import { useActorsView } from "./actors-view-context-provider";
import { useDataProvider } from "./data-provider";
import type { ActorId } from "./queries";

export function ActorNotFound({ actorId }: { actorId?: ActorId }) {
	const { copy } = useActorsView();
	const navigate = useNavigate();
	const hasDevMode = false;

	const { isFetched } = useQuery({
		// biome-ignore lint/style/noNonNullAssertion: enabled guarantees actorId is defined
		...useDataProvider().actorQueryOptions(actorId!),
		enabled: !!actorId,
	});

	// Gate on `isFetched` rather than `isLoading`: the actor query keeps polling
	// on an interval and retries on error, so `isLoading` flips back true on every
	// background refetch and would blank the message to a bare skeleton each cycle.
	// Once the actor has resolved as not-found at least once, keep the message
	// shown steadily. The shimmer skeleton only appears on the genuine first load.
	const isResolved = isFetched;

	return (
		<div className="flex flex-col h-full flex-1">
			<ActorDetailsSkeleton shimmer={!isResolved}>
				<div className="flex text-center text-foreground flex-1 justify-center items-center flex-col gap-2">
					{isResolved ? (
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

					{!hasDevMode && isResolved ? (
						<Button
							className="mt-3"
							variant="outline"
							size="sm"
							onClick={() =>
								navigate({
									to: ".",
									search: (prev) => ({
										...prev,
										devMode: {
											value: ["true"],
											operator: FilterOp.EQUAL,
										},
									}),
								})
							}
						>
							{copy.showHiddenActors}
						</Button>
					) : null}
				</div>
			</ActorDetailsSkeleton>
		</div>
	);
}
