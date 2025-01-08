import {
	Button,
	Flex,
	ScrollArea,
	SmallText,
	Uptime,
	WithTooltip,
} from "@rivet-gg/components";
import { Link } from "@tanstack/react-router";
import type { BackendEvent } from "../../queries";
import { BackendResponseBadge } from "./backend-response-badge";

interface BackendEventsListPanelProps {
	events: BackendEvent[];
	eventId: string | undefined;
}

export function BackendEventsListPanel({
	eventId,
	events,
}: BackendEventsListPanelProps) {
	return (
		<ScrollArea className="overflow-auto h-full truncate min-w-0">
			<Flex
				direction="col"
				gap="2"
				my="4"
				mx="4"
				className="truncate min-w-0"
			>
				{events.map((event) => (
					<Button
						key={event.eventTimestamp}
						variant={
							eventId === event.eventTimestamp
								? "secondary"
								: "outline"
						}
						asChild
					>
						<Link
							to="."
							search={{ eventId: event.eventTimestamp }}
							className="truncate min-w-0"
						>
							<Flex
								gap="2"
								items="center"
								w="full"
								className="truncate min-w-0 max-w-full"
							>
								<BackendResponseBadge {...event} />
								<span className="flex-1 text-left truncate min-w-0 max-w-full inline-block">
									{event.event.request.fmtUrl}
								</span>
								<WithTooltip
									trigger={
										<SmallText>
											<Uptime
												createTs={
													new Date(
														+event.eventTimestamp,
													)
												}
											/>{" "}
											ago
										</SmallText>
									}
									content={event.eventDate}
								/>
							</Flex>
						</Link>
					</Button>
				))}
			</Flex>
		</ScrollArea>
	);
}
