import { faQuestionSquare, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { ShimmerLine } from "../shimmer-line";
import { Button } from "../ui/button";
import { FilterOp } from "../ui/filters";
import { ActorTabs } from "./actors-actor-details";
import { useActorsView } from "./actors-view-context-provider";
import { useManager } from "./manager-context";
import type { ActorFeature, ActorId } from "./queries";

export function ActorNotFound({
	actorId,
	features = [],
}: {
	features?: ActorFeature[];
	actorId?: ActorId;
}) {
	const { copy } = useActorsView();

	const navigate = useNavigate();

	const hasDevMode = false;

	const { isLoading } = useQuery({
		// biome-ignore lint/style/noNonNullAssertion: enabled guarantees actorId is defined
		...useManager().actorQueryOptions(actorId!),
		enabled: !!actorId,
	});

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
								navigate({
									to: ".",
									search: (prev) => ({
										...prev,
										devMode: {
											value: ["true"],
											operator: FilterOp.EQUAL,
										},
									}),
								});
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
