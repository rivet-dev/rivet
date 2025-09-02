import {
	faHammer,
	faLink,
	faMegaphone,
	faTowerBroadcast,
	faUnlink,
	Icon,
} from "@rivet-gg/icons";
import type { RecordedRealtimeEvent } from "@rivetkit/core/inspector";
import { useQuery } from "@tanstack/react-query";
import { format } from "date-fns";
import { type PropsWithChildren, useEffect, useRef } from "react";
import { Badge } from "../ui/badge";
import { useActor } from "./actor-queries-context";
import { ActorObjectInspector } from "./console/actor-inspector";
import type { ActorId } from "./queries";

interface ActorEventsListProps {
	actorId: ActorId;
	search: string;
	filter: string[];
}

export function ActorEventsList({
	actorId,
	search,
	filter,
}: ActorEventsListProps) {
	const actorQueries = useActor();
	const { data, isLoading, isError } = useQuery(
		actorQueries.actorEventsQueryOptions(actorId),
	);

	if (isLoading) {
		return <Info>Loading events...</Info>;
	}

	if (isError) {
		return (
			<Info>
				Realtime Events Preview is currently unavailable.
				<br />
				See console/logs for more details.
			</Info>
		);
	}

	const filteredEvents = data?.events.filter?.((event) => {
		const constraints = [];

		if ("name" in event) {
			constraints.push(
				event.name.toLowerCase().includes(search.toLowerCase()),
			);
		}
		if ("eventName" in event) {
			constraints.push(
				event.eventName.toLowerCase().includes(search.toLowerCase()),
			);
		}
		if (filter.length > 0) {
			const type = event.type.includes("subscribe")
				? "subscription"
				: event.type;
			constraints.push(filter.includes(type));
		}
		return constraints.every(Boolean);
	});

	if (filteredEvents?.length === 0) {
		return <Info>No events found.</Info>;
	}

	return filteredEvents?.map((event) => {
		return <Event {...event} key={event.id} />;
	});
}

function Event(props: RecordedRealtimeEvent) {
	const ref = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (ref.current && props.timestamp > Date.now() - 1000) {
			ref.current.animate(
				[
					{ backgroundColor: "transparent" },
					{ backgroundColor: "hsl(var(--primary) / 15%)" },
					{ backgroundColor: "transparent" },
				],
				{
					duration: 1000,
					fill: "forwards",
					easing: "ease-in-out",
				},
			);
		}
	}, []);

	if (props.type === "action") {
		return (
			<EventContainer ref={ref}>
				<div className="min-h-4 text-foreground/30 flex-shrink-0 [[data-show-timestamps]_&]:block hidden">
					{props.timestamp
						? format(
								props.timestamp,
								"LLL dd HH:mm:ss",
							).toUpperCase()
						: null}
				</div>
				<div className="font-mono-console">
					{props.connId.split("-")[0]}
				</div>
				<div>
					<Badge variant="outline">
						<Icon className="mr-1" icon={faHammer} />
						Action
					</Badge>
				</div>
				<div className="font-mono-console">{props.name}</div>
				<div>
					<ActorObjectInspector data={props.args} />
				</div>
			</EventContainer>
		);
	}
	if (props.type === "subscribe" || props.type === "unsubscribe") {
		return (
			<EventContainer ref={ref}>
				<div className="min-h-4 text-foreground/30 flex-shrink-0 [[data-show-timestamps]_&]:block hidden">
					{props.timestamp
						? format(
								props.timestamp,
								"LLL dd HH:mm:ss",
							).toUpperCase()
						: null}
				</div>
				<div className="font-mono-console">
					{props.connId.split("-")[0]}
				</div>
				<div>
					<Badge variant="outline">
						<Icon
							className="mr-1"
							icon={
								props.type === "subscribe" ? faLink : faUnlink
							}
						/>
						{props.type === "subscribe"
							? "Subscribe"
							: "Unsubscribe"}
					</Badge>
				</div>
				<div className="font-mono-console">{props.eventName}</div>
				<div />
			</EventContainer>
		);
	}
	if (props.type === "broadcast") {
		return (
			<EventContainer ref={ref}>
				<div className="min-h-4 text-foreground/30 flex-shrink-0 [[data-show-timestamps]_&]:block hidden">
					{props.timestamp
						? format(
								props.timestamp,
								"LLL dd HH:mm:ss",
							).toUpperCase()
						: null}
				</div>
				<div />
				<div>
					<Badge variant="outline">
						<Icon className="mr-1" icon={faTowerBroadcast} />
						Broadcast
					</Badge>
				</div>
				<div className="font-mono-console">{props.eventName}</div>
				<div>
					<ActorObjectInspector data={props.args} />
				</div>
			</EventContainer>
		);
	}
	if (props.type === "event") {
		return (
			<EventContainer ref={ref}>
				<div className="min-h-4 text-foreground/30 flex-shrink-0 [[data-show-timestamps]_&]:block hidden">
					{props.timestamp
						? format(
								props.timestamp,
								"LLL dd HH:mm:ss",
							).toUpperCase()
						: null}
				</div>
				<div className="font-mono-console">
					{props.connId.split("-")[0]}
				</div>
				<div>
					<Badge variant="outline">
						<Icon className="mr-1" icon={faMegaphone} />
						Send
					</Badge>
				</div>
				<div className="font-mono-console">{props.eventName}</div>
				<div>
					<ActorObjectInspector data={props.args} />
				</div>
			</EventContainer>
		);
	}
}

function EventContainer({
	ref,
	children,
}: {
	ref: React.RefObject<HTMLDivElement>;
	children: React.ReactNode;
}) {
	return (
		<div
			ref={ref}
			className="grid grid-cols-subgrid col-span-full gap-2 px-4 py-2 border-b text-xs items-center"
		>
			{children}
		</div>
	);
}

function Info({ children }: PropsWithChildren) {
	return (
		<div className="flex-1 flex flex-col gap-2 items-center justify-center h-full text-center col-span-full py-8 text-xs">
			{children}
		</div>
	);
}
