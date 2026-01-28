import { faSpinnerThird, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { format } from "date-fns";
import { LiveBadge, ScrollArea } from "@/components";
import { useActorInspector } from "./actor-inspector-context";
import type { ActorId } from "./queries";

const DEFAULT_QUEUE_LIMIT = 200;

export function ActorQueue({ actorId }: { actorId: ActorId }) {
	const inspector = useActorInspector();
	const queueStatusQuery = useQuery({
		...inspector.actorQueueStatusQueryOptions(actorId, DEFAULT_QUEUE_LIMIT),
		enabled:
			inspector.isInspectorAvailable &&
			inspector.features.queue.supported,
		refetchInterval: 1000,
		refetchOnWindowFocus: false,
	});
	const queueSizeQuery = useQuery(
		inspector.actorQueueSizeQueryOptions(actorId),
	);

	if (queueStatusQuery.isLoading) {
		return (
			<div className="flex-1 flex items-center justify-center text-sm text-muted-foreground">
				<Icon icon={faSpinnerThird} className="animate-spin mr-2" />
				Loading queue...
			</div>
		);
	}

	if (queueStatusQuery.isError || !queueStatusQuery.data) {
		return (
			<div className="flex-1 flex items-center justify-center text-sm text-muted-foreground">
				Queue data is currently unavailable.
			</div>
		);
	}

	const status = queueStatusQuery.data;
	const size = Number.isFinite(status.size)
		? status.size
		: (queueSizeQuery.data ?? 0);

	return (
		<ScrollArea className="flex-1 w-full min-h-0 h-full">
			<div className="flex justify-between items-center gap-2 border-b sticky top-0 p-2 z-[1] h-[45px]">
				<LiveBadge />
				<div className="text-xs text-muted-foreground">
					Queue size {size} / {status.maxSize}
				</div>
			</div>
			<div className="p-3 space-y-2">
				{status.messages.length === 0 ? (
					<div className="text-sm text-muted-foreground">
						Queue is empty.
					</div>
				) : (
					status.messages.map((message) => (
						<div
							key={message.id}
							className="rounded-md border border-border/60 bg-background px-3 py-2 text-xs"
						>
							<div className="flex items-center justify-between gap-3">
								<div className="font-medium text-foreground">
									{message.name}
								</div>
								<div className="text-muted-foreground">
									{format(new Date(message.createdAtMs), "p")}
								</div>
							</div>
							<div className="mt-1 text-[11px] text-muted-foreground">
								ID {message.id}
							</div>
						</div>
					))
				)}
				{status.truncated ? (
					<div className="text-xs text-muted-foreground">
						Showing the first {DEFAULT_QUEUE_LIMIT} messages.
					</div>
				) : null}
			</div>
		</ScrollArea>
	);
}
