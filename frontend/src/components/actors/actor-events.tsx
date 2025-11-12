import { faPause, faPlay, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import {
	startTransition,
	useCallback,
	useEffect,
	useRef,
	useState,
} from "react";
import { useResizeObserver } from "usehooks-ts";
import {
	Button,
	LiveBadge,
	LogsView,
	PauseBadge,
	ScrollArea,
	ToggleGroup,
	ToggleGroupItem,
	WithTooltip,
} from "@/components";
import { ActorClearEventsLogButton } from "./actor-clear-events-log-button";
import { useActorDetailsSettings } from "./actor-details-settings";
import { ActorDetailsSettingsButton } from "./actor-details-settings-button";
import { ActorEventsList } from "./actor-events-list";
import {
	type TransformedInspectorEvent,
	useActorInspector,
} from "./actor-inspector-context";
import type { ActorId } from "./queries";

export type EventsTypeFilter = TransformedInspectorEvent["body"]["tag"];

interface ActorEventsProps {
	actorId: ActorId;
}

export function ActorEvents({ actorId }: ActorEventsProps) {
	const [search, setSearch] = useState("");
	const [logsFilter, setLogsFilter] = useState<EventsTypeFilter[]>([
		"ActionEvent",
		"SubscribeEvent",
		"BroadcastEvent",
		"FiredEvent",
	]);

	const ref = useRef<HTMLDivElement>(null);
	const [settings] = useActorDetailsSettings();

	const actorQueries = useActorInspector();
	const { data } = useQuery(actorQueries.actorEventsQueryOptions(actorId));
	const { onScroll } = useScrollToBottom(ref, [data]);

	return (
		<div className="flex flex-col h-full">
			<div className="border-b">
				<div className="flex items-stretch px-2">
					<div className="border-r flex flex-1">
						<input
							type="text"
							className="bg-transparent outline-none px-2 text-xs placeholder:text-muted-foreground font-sans flex-1"
							placeholder="Filter output"
							spellCheck={false}
							onChange={(e) =>
								startTransition(() => setSearch(e.target.value))
							}
						/>
					</div>
					<ToggleGroup
						type="multiple"
						value={logsFilter}
						size="xs"
						onValueChange={(value) => {
							setLogsFilter(value as EventsTypeFilter[]);
						}}
						className="gap-0 text-xs p-2 border-r"
					>
						<ToggleGroupItem
							value="ActionEvent"
							className="text-xs border border-r-0 rounded-se-none rounded-ee-none"
						>
							Action
						</ToggleGroupItem>
						<ToggleGroupItem
							value="SubscribeEvent"
							className="text-xs border rounded-none"
						>
							Subscription
						</ToggleGroupItem>
						<ToggleGroupItem
							value="BroadcastEvent"
							className="text-xs border rounded-none"
						>
							Broadcast
						</ToggleGroupItem>
						<ToggleGroupItem
							value="FiredEvent"
							className=" text-xs border rounded-es-none rounded-ss-none border-l-0"
						>
							Send
						</ToggleGroupItem>
					</ToggleGroup>
					<div className="flex items-center gap-2 pl-2">
						<ActorDetailsSettingsButton />
						<ActorClearEventsLogButton actorId={actorId} />

						<div className="h-full flex items-center">
							<LiveBadge />
						</div>
					</div>
				</div>
			</div>
			<div className="flex-1 min-h-0 overflow-hidden flex relative">
				<ScrollArea
					viewportRef={ref}
					viewportProps={{ onScroll }}
					className="w-full h-full min-h-0"
				>
					<div
						data-show-timestamps={
							settings.showTimestamps ? "" : undefined
						}
						className="grid grid-cols-[1fr_1fr_1fr_2fr] [&[data-show-timestamps]]:grid-cols-[1fr_1fr_1fr_1fr_2fr] auto-rows-min w-full h-full min-h-0"
					>
						<div className="grid grid-cols-subgrid col-span-full font-semibold text-xs px-4 pr-4 h-[45px] items-center border-b">
							<div className="[[data-show-timestamps]_&]:block hidden">
								Timestamp
							</div>
							<div>Connection</div>
							<div>Event</div>
							<div>Name</div>
							<div>Data</div>
						</div>

						<ActorEventsList
							search={search}
							filter={logsFilter}
							actorId={actorId}
						/>
					</div>
				</ScrollArea>
			</div>
		</div>
	);
}

ActorEvents.Skeleton = () => {
	return (
		<div className="px-4 pt-4">
			<LogsView.Skeleton />
		</div>
	);
};

function useScrollToBottom(
	ref: React.RefObject<HTMLDivElement | null>,
	deps: unknown[],
) {
	const [settings] = useActorDetailsSettings();
	const [follow, setFollow] = useState(true);
	const shouldFollow = () => settings.autoFollowLogs && follow;
	const shouldScanForNew = useRef(false);
	useResizeObserver({
		// @ts-expect-error -- TS2322 -- Type 'HTMLDivElement' is not assignable to type 'Element | null'.
		ref,
		onResize: () => {
			if (shouldFollow()) {
				// https://github.com/TanStack/virtual/issues/537
				requestAnimationFrame(() => {
					ref.current?.scrollTo({
						top: ref.current.scrollHeight,
						behavior: "instant",
					});
				});
			}
		},
	});

	const onScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
		if (shouldScanForNew.current) {
			return;
		}
		setFollow(
			e.currentTarget.scrollHeight - e.currentTarget.scrollTop <=
				e.currentTarget.clientHeight,
		);
	}, []);

	useEffect(
		() => {
			if (!shouldFollow()) {
				return () => {};
			}
			shouldScanForNew.current = true;
			// https://github.com/TanStack/virtual/issues/537
			const rafId = requestAnimationFrame(() => {
				ref.current?.scrollTo({
					top: ref.current.scrollHeight,
					behavior: "instant",
				});
				shouldScanForNew.current = false;
			});

			return () => {
				cancelAnimationFrame(rafId);
				shouldScanForNew.current = false;
			};
		},
		// biome-ignore lint/correctness/useExhaustiveDependencies: deps is passed from caller
		deps,
	);

	return { onScroll };
}
