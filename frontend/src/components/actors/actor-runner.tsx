import { useQuery } from "@tanstack/react-query";
import { Row } from "@/app/runners-table";
import { useDataProvider } from "./data-provider";
import type { ActorId } from "./queries";

export interface ActorRunnerProps {
	actorId: ActorId;
}

export function ActorRunner({ actorId }: ActorRunnerProps) {
	const { data: actor } = useQuery(
		useDataProvider().actorQueryOptions(actorId),
	);

	const { data: runner } = useQuery({
		...useDataProvider().runnerByNameQueryOptions({
			runnerName: actor?.runnerNameSelector || "",
		}),
		enabled: !!actor?.runnerNameSelector,
	});

	if (!runner) {
		return null;
	}

	return (
		<div className="px-4 mt-4 mb-8">
			<h3 className="mb-2 font-semibold">Runner</h3>

			<Row {...runner} />
		</div>
	);
}
