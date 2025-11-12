import {
	faHammer,
	faLink,
	faMegaphone,
	faTowerBroadcast,
	Icon,
} from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { format } from "date-fns";
import { type PropsWithChildren, useEffect, useRef } from "react";
import { match, P } from "ts-pattern";
import { Badge } from "../ui/badge";
import {
	type TransformedInspectorEvent,
	useActorInspector,
} from "./actor-inspector-context";
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
	const actorInspector = useActorInspector();
	const { data, isLoading, isError } = useQuery(
		actorInspector.actorEventsQueryOptions(actorId),
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

	const filteredEvents = data?.filter?.((event) => {
		const constraints = [];

		if ("name" in event.body.val) {
			constraints.push(
				event.body.val.name
					.toLowerCase()
					.includes(search.toLowerCase()),
			);
		}
		if ("eventName" in event.body.val) {
			constraints.push(
				event.body.val.eventName
					.toLowerCase()
					.includes(search.toLowerCase()),
			);
		}
		if (filter.length > 0) {
			const type = event.body.tag;
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

function Event(props: TransformedInspectorEvent) {
	const ref = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (ref.current && props.timestamp.getTime() > Date.now() - 1000) {
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
	}, [props.timestamp]);

	return match(props.body)
		.with({ tag: "ActionEvent" }, (body) => {
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
						{body.val.connId.split("-")[0]}
					</div>
					<div>
						<Badge variant="outline">
							<Icon className="mr-1" icon={faHammer} />
							Action
						</Badge>
					</div>
					<div className="font-mono-console">{body.val.name}</div>
					<div>
						<ActorObjectInspector data={body.val.args} />
					</div>
				</EventContainer>
			);
		})
		.with(
			{ tag: P.union("SubscribeEvent", "UnSubscribeEvent") },
			(body) => {
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
							{body.val.connId.split("-")[0]}
						</div>
						<div>
							<Badge variant="outline">
								<Icon className="mr-1" icon={faLink} />
								{body.tag === "SubscribeEvent"
									? "Subscribe"
									: "Unsubscribe"}
							</Badge>
						</div>
						<div className="font-mono-console">
							{body.val.eventName}
						</div>
						<div />
					</EventContainer>
				);
			},
		)
		.with({ tag: "BroadcastEvent" }, (body) => {
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
					<div className="font-mono-console">
						{body.val.eventName}
					</div>
					<div>
						<ActorObjectInspector data={body.val.args} />
					</div>
				</EventContainer>
			);
		})
		.with({ tag: "FiredEvent" }, (body) => {
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
						{body.val.connId.split("-")[0]}
					</div>
					<div>
						<Badge variant="outline">
							<Icon className="mr-1" icon={faMegaphone} />
							Send
						</Badge>
					</div>
					<div className="font-mono-console">
						{body.val.eventName}
					</div>
					<div>
						<ActorObjectInspector data={body.val.args} />
					</div>
				</EventContainer>
			);
		})
		.exhaustive();
}

function EventContainer({
	ref,
	children,
}: {
	ref: React.RefObject<HTMLDivElement | null>;
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
