import type { Rivet } from "@rivet-gg/api";
import { VirtualScrollArea } from "@rivet-gg/components";
import { useQueries } from "@tanstack/react-query";
import type { Virtualizer } from "@tanstack/react-virtual";
import { memo, useCallback, useEffect, useRef } from "react";
import { useResizeObserver } from "usehooks-ts";
import { actorLogsQueryOptions } from "../../queries";
import { useActorDetailsSettings } from "./actor-details-settings";
import { getActorStatus } from "./actor-status-indicator";
import { ActorConsoleMessage } from "./console/actor-console-message";

export type LogsTypeFilter = "all" | "output" | "errors";

type Logs = {
	lines: string[];
	timestamps: string[];
	ids: string[];
};

export function filterLogs({
	typeFilter,
	filter,
	logs,
	errors,
}: { typeFilter: LogsTypeFilter; filter: string; logs: Logs; errors: Logs }) {
	const stdOutput =
		typeFilter === "errors"
			? []
			: (logs?.lines.map((log, index) => ({
					variant: log.includes("level=WARN")
						? ("warn" as const)
						: ("log" as const),
					message: log,
					timestamp: logs.timestamps[index],
					id: logs.ids[index],
				})) ?? []);

	const errOutput =
		typeFilter === "output"
			? []
			: (errors?.lines.map((log, index) => ({
					variant: "error" as const,
					message: log,
					timestamp: errors.timestamps[index],
					id: errors.ids[index],
				})) ?? []);

	const output = [...stdOutput, ...errOutput];

	// Search
	const filtered =
		filter && filter.trim() !== ""
			? output.filter((log) => log.message.includes(filter))
			: output;

	const sorted = filtered.toSorted(
		(a, b) =>
			new Date(a.timestamp).valueOf() - new Date(b.timestamp).valueOf(),
	);

	return sorted;
}
interface ActorLogsProps
	extends Pick<Rivet.actor.Actor, "createdAt" | "startedAt" | "destroyedAt"> {
	actorId: string;
	projectNameId: string;
	environmentNameId: string;
	typeFilter?: LogsTypeFilter;
	filter?: string;
}

export const ActorLogs = memo(
	({
		actorId,
		projectNameId,
		environmentNameId,
		typeFilter,
		filter,

		createdAt,
		startedAt,
		destroyedAt,
	}: ActorLogsProps) => {
		const [settings] = useActorDetailsSettings();
		const follow = useRef(true);
		const shouldFollow = () => settings.autoFollowLogs && follow.current;

		const viewport = useRef<HTMLDivElement>(null);
		const virtualizer = useRef<Virtualizer<HTMLDivElement, Element>>(null);
		// Detect if the container has resized (i.e, console was opened)
		useResizeObserver({
			ref: viewport,
			onResize: () => {
				if (shouldFollow()) {
					// https://github.com/TanStack/virtual/issues/537
					requestAnimationFrame(() => {
						virtualizer.current?.scrollToIndex(combined.length, {
							align: "end",
						});
					});
				}
			},
		});
		const [
			{
				data: logs,
				isSuccess: isStdOutSuccess,
				isLoading: isStdOutLoading,
			},
			{
				data: errors,
				isSuccess: isStdErrSuccess,
				isLoading: isStdErrLoading,
			},
		] = useQueries({
			queries: [
				actorLogsQueryOptions({
					actorId,
					projectNameId,
					environmentNameId,
					stream: "std_out",
				}),
				actorLogsQueryOptions({
					actorId,
					projectNameId,
					environmentNameId,
					stream: "std_err",
				}),
			],
		});

		const combined = filterLogs({
			typeFilter: typeFilter ?? "all",
			filter: filter ?? "",
			logs: logs as Logs,
			errors: errors as Logs,
		});

		// Scroll to the bottom when new logs are added
		// biome-ignore lint/correctness/useExhaustiveDependencies: run this effect only when the length of the logs changes
		useEffect(() => {
			if (!shouldFollow()) {
				return () => {};
			}
			// https://github.com/TanStack/virtual/issues/537
			const rafId = requestAnimationFrame(() => {
				virtualizer.current?.scrollToIndex(
					virtualizer.current.options.count - 1,
					{
						align: "end",
					},
				);
			});

			return () => {
				cancelAnimationFrame(rafId);
			};
		}, [combined.length]);

		// Detect if the user has scrolled all the way to the bottom
		const handleChange = useCallback(
			(instance: Virtualizer<HTMLDivElement, Element>, sync: boolean) => {
				if (sync) {
					return;
				}

				follow.current =
					!instance.isScrolling &&
					instance.range?.endIndex === instance.options.count - 1;
			},
			[],
		);

		if (isStdOutLoading || isStdErrLoading) {
			return (
				<div className="w-full flex-1 min-h-0">
					<ActorConsoleMessage variant="warn">
						Loading logs...
					</ActorConsoleMessage>
				</div>
			);
		}

		const status = getActorStatus({ createdAt, startedAt, destroyedAt });

		if (status === "starting" && combined.length === 0) {
			return (
				<div className="w-full flex-1 min-h-0">
					<ActorConsoleMessage variant="debug">
						[SYSTEM]: Actor is starting...
					</ActorConsoleMessage>
				</div>
			);
		}

		if (combined.length === 0) {
			if (!isStdOutSuccess || !isStdErrSuccess) {
				return (
					<div className="w-full flex-1 min-h-0">
						<ActorConsoleMessage variant="error">
							[SYSTEM]: Couldn't find the logs. Please try again
							later.
						</ActorConsoleMessage>
					</div>
				);
			}
			return (
				<div className="w-full flex-1 min-h-0">
					<ActorConsoleMessage variant="debug">
						[SYSTEM]: No logs found. Logs are retained for 3 days.
					</ActorConsoleMessage>
				</div>
			);
		}

		return (
			<>
				<Scroller key={actorId} virtualizer={virtualizer} />
				<VirtualScrollArea
					viewportRef={viewport}
					virtualizerRef={virtualizer}
					className="w-full flex-1 min-h-0"
					getRowData={(index) => ({
						...combined[index],
						children: combined[index].message,
						timestamp: settings.showTimestmaps
							? combined[index].timestamp
							: undefined,
					})}
					onChange={handleChange}
					count={combined.length}
					estimateSize={() => 26}
					row={ActorConsoleMessage}
				/>
			</>
		);
	},
);

interface ScrollerProps {
	virtualizer: React.MutableRefObject<Virtualizer<
		HTMLDivElement,
		Element
	> | null>;
}

function Scroller({ virtualizer }: ScrollerProps) {
	// biome-ignore lint/correctness/useExhaustiveDependencies: scroll on mount, no need to run this effect again
	useEffect(() => {
		// https://github.com/TanStack/virtual/issues/537
		virtualizer.current?.scrollToIndex(
			virtualizer.current.options.count - 1,
			{
				align: "end",
			},
		);
	}, []);

	return null;
}
