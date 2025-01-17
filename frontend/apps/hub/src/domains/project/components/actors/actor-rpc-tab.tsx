import type { Rivet } from "@rivet-gg/api";
import { Accordion, ScrollArea } from "@rivet-gg/components";
import { useQuery } from "@tanstack/react-query";
import { actorsRpcsQueryOptions } from "../../queries";
import { ActorRpc } from "./actor-rpc";

interface ActorRpcTabProps extends Rivet.actor.Actor {
	projectNameId: string;
	environmentNameId: string;
}

export function ActorRpcTab({
	network,
	projectNameId,
	environmentNameId,
	id: actorId,
}: ActorRpcTabProps) {
	const { data } = useQuery(
		actorsRpcsQueryOptions({
			actorId,
			network,
			projectNameId,
			environmentNameId,
		}),
	);

	return (
		<ScrollArea className="overflow-auto h-full px-4 my-2">
			{data?.length === 0 ? (
				<p className="my-8 text-center text-sm text-muted-foreground">
					No RPCs found.
				</p>
			) : (
				<Accordion type="multiple">
					{data?.map((rpc) => (
						<ActorRpc key={rpc} rpc={rpc} />
					))}
				</Accordion>
			)}
		</ScrollArea>
	);
}
