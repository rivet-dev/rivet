import type { Rivet } from "@rivet-gg/api";
import {
	Code,
	JsonCode,
	Link,
	ScrollArea,
	Skeleton,
} from "@rivet-gg/components";
import { useQuery } from "@tanstack/react-query";

import { actorStateQueryOptions } from "../../queries";

interface ActorStateTabProps extends Rivet.actor.Actor {
	projectNameId: string;
	environmentNameId: string;
}

export function ActorStateTab({
	id: actorId,
	network,
	projectNameId,
	environmentNameId,
}: ActorStateTabProps) {
	const { data, isLoading, error } = useQuery(
		actorStateQueryOptions({
			actorId,
			network,
			projectNameId,
			environmentNameId,
		}),
	);

	if (error) {
		return (
			<ScrollArea className="overflow-auto h-full px-4 my-2">
				<p className="my-8 text-center text-sm text-muted-foreground">
					Failed to fetch state data.
					<br />
					If this issue persists, please contact support.
				</p>
			</ScrollArea>
		);
	}

	return (
		<ScrollArea className="overflow-auto h-full px-4 my-2">
			{isLoading ? (
				<Skeleton className="w-full h-80" />
			) : !data?.enabled ? (
				<p className="my-8 text-center text-sm text-muted-foreground">
					State functionality is not enabled for this actor.
					<br /> Enable it by adding <Code>_onInitialize</Code>{" "}
					method.{" "}
					<Link
						href="https://rivet.gg/docs/state"
						rel="noopener noreferrer"
						target="_blank"
					>
						Learn more
					</Link>
					.
				</p>
			) : (
				<>
					<JsonCode value={data?.native || ""} readOnly />
					<p className="text-xs text-muted-foreground my-2">
						State is fetched every second. You can override the data
						shown above by implementing
						<Code className="text-xs">_inspectState</Code> method in
						your actor.{" "}
						<Link
							href="https://rivet.gg/docs/state#debugging"
							rel="noopener noreferrer"
							target="_blank"
						>
							Learn more
						</Link>
						.
					</p>
				</>
			)}
		</ScrollArea>
	);
}
