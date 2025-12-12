import { useQuery } from "@tanstack/react-query";
import {
	startTransition,
	useCallback,
	useEffect,
	useRef,
	useState,
} from "react";
import { useResizeObserver } from "usehooks-ts";
import { ScrollArea } from "@/components";
import { useActorDetailsSettings } from "./actor-details-settings";
import { ActorKvList } from "./actor-kv-list";
import { useActor } from "./actor-queries-context";
import type { ActorId } from "./queries";

interface ActorKvProps {
	actorId: ActorId;
}

export function ActorKv({ actorId }: ActorKvProps) {
	const [search, setSearch] = useState("");
	const ref = useRef<HTMLDivElement>(null);
	const [settings] = useActorDetailsSettings();

	const actorQueries = useActor();
	const { data } = useQuery(actorQueries.actorKvQueryOptions(actorId));
	const { onScroll } = useScrollToBottom(ref, [data]);

	return (
		<div className="flex flex-col h-full">
			<div className="border-b">
				<div className="flex items-stretch px-2">
					<div className="border-r flex flex-1">
						<input
							type="text"
							className="bg-transparent outline-none px-2 text-xs placeholder:text-muted-foreground font-sans flex-1"
							placeholder="Search by key"
							spellCheck={false}
							onChange={(e) =>
								startTransition(() => setSearch(e.target.value))
							}
						/>
					</div>
				</div>
			</div>
			<div className="flex-1 min-h-0 overflow-hidden flex relative">
				<ScrollArea
					viewportRef={ref}
					onScroll={onScroll}
					className="w-full h-full min-h-0"
				>
					<div
						data-show-timestamps={
							settings.showTimestamps ? "" : undefined
						}
						className="grid grid-cols-[2fr_3fr_1fr] [&[data-show-timestamps]]:grid-cols-[1fr_2fr_3fr_1fr] auto-rows-min w-full h-full min-h-0"
					>
						<div className="grid grid-cols-subgrid col-span-full font-semibold text-xs px-4 pr-4 h-[45px] items-center border-b">
							<div className="[[data-show-timestamps]_&]:block hidden">
								Updated At
							</div>
							<div>Key</div>
							<div>Value</div>
							<div>Size</div>
						</div>

						<ActorKvList
							search={search}
							actorId={actorId}
						/>
					</div>
				</ScrollArea>
			</div>
		</div>
	);
}

function useScrollToBottom(
	ref: React.RefObject<HTMLDivElement | null>,
	deps: unknown[],
) {
	const [settings] = useActorDetailsSettings();
	const follow = useRef(true);
	const shouldFollow = () => settings.autoFollowLogs && follow.current;
	useResizeObserver({
		// @ts-expect-error -- TS2322 -- Type 'HTMLDivElement' is not assignable to type 'Element | null'.
		ref,
		onResize: () => {
			if (shouldFollow()) {
				// https://github.com/TanStack/virtual/issues/537
				requestAnimationFrame(() => {
					ref.current?.scrollTo({
						top: ref.current.scrollHeight,
						behavior: "smooth",
					});
				});
			}
		},
	});

	const onScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
		follow.current =
			e.currentTarget.scrollHeight - e.currentTarget.scrollTop <=
			e.currentTarget.clientHeight;
	}, []);

	useEffect(
		() => {
			if (!shouldFollow()) {
				return () => {};
			}
			// https://github.com/TanStack/virtual/issues/537
			const rafId = requestAnimationFrame(() => {
				ref.current?.scrollTo({
					top: ref.current.scrollHeight,
					behavior: "smooth",
				});
			});

			return () => {
				cancelAnimationFrame(rafId);
			};
		},
		// biome-ignore lint/correctness/useExhaustiveDependencies: deps is passed from caller
		deps,
	);

	return { onScroll };
}
