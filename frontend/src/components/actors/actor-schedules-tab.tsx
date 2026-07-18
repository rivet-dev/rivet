import {
	faCalendar,
	faClock,
	faSpinnerThird,
	faTrash,
	Icon,
} from "@rivet-gg/icons";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
	Sheet,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
} from "@/components/ui/sheet";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";
import { cn } from "../lib/utils";
import {
	type InspectorSchedule,
	type InspectorScheduleFire,
	useActorInspector,
} from "./actor-inspector-context";
import { formatDuration, formatSchedule } from "./actor-schedules-format";
import type { ActorId } from "./queries";

type ScheduleFilter = "all" | "one-time" | "recurring";

export function ActorSchedulesTab({ actorId }: { actorId: ActorId }) {
	const inspector = useActorInspector();
	const [search, setSearch] = useState("");
	const [filter, setFilter] = useState<ScheduleFilter>("all");
	const [selectedId, setSelectedId] = useState<string>();
	const [now, setNow] = useState(() => Date.now());

	const { data: schedules = [], isLoading } = useQuery(
		inspector.actorSchedulesQueryOptions(actorId),
	);

	useEffect(() => {
		const timer = window.setInterval(() => setNow(Date.now()), 1_000);
		return () => window.clearInterval(timer);
	}, []);

	const filtered = useMemo(() => {
		const query = search.trim().toLowerCase();
		return schedules.filter((schedule) => {
			if (filter === "one-time" && schedule.kind !== "at") return false;
			if (filter === "recurring" && schedule.kind === "at") return false;
			if (!query) return true;
			return [
				schedule.name,
				schedule.id,
				schedule.action,
				schedule.expression,
				schedule.timezone,
			]
				.filter(Boolean)
				.some((value) => value?.toLowerCase().includes(query));
		});
	}, [filter, schedules, search]);

	const selected = schedules.find((schedule) => schedule.id === selectedId);

	if (!inspector.features.schedules.supported) {
		return (
			<div className="flex flex-1 items-center justify-center p-8 text-sm text-muted-foreground">
				{inspector.features.schedules.message}
			</div>
		);
	}

	return (
		<div className="flex min-h-0 flex-1 flex-col" data-testid="schedules-tab">
			<div className="flex flex-wrap items-center gap-2 border-b px-4 py-3">
				<div className="mr-auto flex items-center gap-2">
					<h2 className="font-semibold">Schedules</h2>
					<Badge variant="secondary">
						{schedules.length} active
					</Badge>
				</div>
				<div className="flex rounded-md border bg-muted/30 p-0.5">
					{(["all", "one-time", "recurring"] as const).map((value) => (
						<Button
							key={value}
							variant="ghost"
							size="sm"
							className={cn(
								"h-7 px-2.5 text-xs capitalize",
								filter === value && "bg-background shadow-sm",
							)}
							onClick={() => setFilter(value)}
						>
							{value}
						</Button>
					))}
				</div>
				<Input
					value={search}
					onChange={(event) => setSearch(event.target.value)}
					placeholder="Search schedules…"
					className="h-8 w-48"
				/>
			</div>

			{isLoading ? (
				<div className="flex flex-1 items-center justify-center text-muted-foreground">
					<Icon icon={faSpinnerThird} className="mr-2 animate-spin" />
					Loading schedules…
				</div>
			) : schedules.length === 0 ? (
				<EmptySchedules />
			) : filtered.length === 0 ? (
				<div className="flex flex-1 items-center justify-center p-8 text-sm text-muted-foreground">
					No schedules match this filter.
				</div>
			) : (
				<Table containerClassName="flex-1 min-h-0">
					<TableHeader className="sticky top-0 z-10 bg-background">
						<TableRow>
							<TableHead>Type</TableHead>
							<TableHead>Name / ID</TableHead>
							<TableHead>Action</TableHead>
							<TableHead>Schedule</TableHead>
							<TableHead>Next run</TableHead>
							<TableHead>Last start</TableHead>
						</TableRow>
					</TableHeader>
					<TableBody>
						{filtered.map((schedule) => (
							<TableRow
								key={`${schedule.kind}:${schedule.id}`}
								isClickable
								onClick={() => setSelectedId(schedule.id)}
								data-testid={`schedule-row-${schedule.id}`}
							>
								<TableCell>
									<KindBadge kind={schedule.kind} />
								</TableCell>
								<TableCell className="max-w-56">
									<div className="truncate font-medium">
										{schedule.name ?? schedule.id}
									</div>
									{schedule.name && (
										<div className="truncate font-mono text-[11px] text-muted-foreground">
											{schedule.id}
										</div>
									)}
								</TableCell>
								<TableCell className="font-mono text-xs">
									{schedule.action}
								</TableCell>
								<TableCell>{formatSchedule(schedule)}</TableCell>
								<TableCell>
									<RelativeTime timestamp={schedule.nextRunAt} now={now} />
								</TableCell>
								<TableCell className="text-muted-foreground">
									{schedule.lastRunAt ? (
										<RelativeTime timestamp={schedule.lastRunAt} now={now} />
									) : (
										"—"
									)}
								</TableCell>
							</TableRow>
						))}
					</TableBody>
				</Table>
			)}

			<ScheduleDetails
				actorId={actorId}
				schedule={selected}
				now={now}
				onClose={() => setSelectedId(undefined)}
			/>
		</div>
	);
}

function EmptySchedules() {
	return (
		<div className="flex flex-1 flex-col items-center justify-center p-8 text-center">
			<div className="mb-3 flex size-11 items-center justify-center rounded-full bg-muted">
				<Icon icon={faCalendar} className="text-muted-foreground" />
			</div>
			<h3 className="font-medium">No schedules yet</h3>
			<p className="mt-1 max-w-sm text-sm text-muted-foreground">
				One-time and recurring schedules created by this actor will appear
				here.
			</p>
		</div>
	);
}

function ScheduleDetails({
	actorId,
	schedule,
	now,
	onClose,
}: {
	actorId: ActorId;
	schedule: InspectorSchedule | undefined;
	now: number;
	onClose: () => void;
}) {
	const inspector = useActorInspector();
	const [confirming, setConfirming] = useState(false);
	const isRecurring = schedule?.kind !== "at";
	const { data: history = [], isLoading } = useQuery({
		...inspector.actorScheduleHistoryQueryOptions(
			actorId,
			schedule?.id ?? "",
		),
		enabled: Boolean(schedule && isRecurring),
	});
	const deletion = useMutation({
		...inspector.actorScheduleDeleteMutation(actorId),
		onSuccess: (deleted) => {
			if (deleted) toast.success("Schedule deleted");
			onClose();
		},
		onError: (error) => toast.error(error.message),
	});

	useEffect(() => setConfirming(false), [schedule?.id]);

	return (
		<Sheet open={Boolean(schedule)} onOpenChange={(open) => !open && onClose()}>
			<SheetContent className="flex w-full flex-col sm:max-w-lg">
				{schedule && (
					<>
						<SheetHeader className="border-b pb-4 pr-6">
							<div className="flex items-center gap-2">
								<SheetTitle className="truncate">
									{schedule.name ?? "One-time schedule"}
								</SheetTitle>
								<KindBadge kind={schedule.kind} />
							</div>
							<SheetDescription className="truncate font-mono text-xs">
								{schedule.id}
							</SheetDescription>
						</SheetHeader>

						<div className="flex-1 space-y-6 overflow-y-auto py-4">
							<dl className="grid grid-cols-[7rem_1fr] gap-x-4 gap-y-3 text-sm">
								<Detail label="Action">
									<code>{schedule.action}</code>
								</Detail>
								<Detail label="Schedule">{formatSchedule(schedule)}</Detail>
								<Detail label="Next run">
									<div>{formatTimestamp(schedule.nextRunAt)}</div>
									<div className="text-xs text-muted-foreground">
										<RelativeTime timestamp={schedule.nextRunAt} now={now} />
									</div>
								</Detail>
								{schedule.lastRunAt && (
									<Detail label="Last start">
										{formatTimestamp(schedule.lastRunAt)}
									</Detail>
								)}
								{schedule.maxHistory != null && (
									<Detail label="History">
										{schedule.maxHistory === 0
											? "Disabled"
											: `Keep ${schedule.maxHistory} entries`}
									</Detail>
								)}
							</dl>

							<section>
								<h3 className="mb-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
									Arguments
								</h3>
								<pre className="max-h-48 overflow-auto rounded-md border bg-muted/30 p-3 text-xs">
									{JSON.stringify(schedule.args, null, 2)}
								</pre>
							</section>

							{isRecurring && (
								<section>
									<h3 className="mb-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
										Recent runs
									</h3>
									{isLoading ? (
										<div className="py-6 text-center text-sm text-muted-foreground">
											<Icon
												icon={faSpinnerThird}
												className="mr-2 animate-spin"
											/>
											Loading history…
										</div>
									) : history.length ? (
										<HistoryList history={history} now={now} />
									) : (
										<div className="rounded-md border border-dashed p-5 text-center text-sm text-muted-foreground">
											No runs recorded yet.
										</div>
									)}
								</section>
							)}
						</div>

						<div className="border-t pt-4">
							<Button
								variant={confirming ? "destructive" : "outline"}
								className="w-full"
								disabled={deletion.isPending}
								onClick={() => {
									if (!confirming) {
										setConfirming(true);
										return;
									}
									deletion.mutate({
										scheduleId: schedule.id,
										kind: schedule.kind,
									});
								}}
								startIcon={
									<Icon
										icon={deletion.isPending ? faSpinnerThird : faTrash}
										className={cn(deletion.isPending && "animate-spin")}
									/>
								}
							>
								{confirming
									? schedule.kind === "at"
										? "Confirm cancellation"
										: "Confirm deletion"
									: schedule.kind === "at"
										? "Cancel schedule"
										: "Delete schedule"}
							</Button>
						</div>
					</>
				)}
			</SheetContent>
		</Sheet>
	);
}

function Detail({ label, children }: { label: string; children: React.ReactNode }) {
	return (
		<>
			<dt className="text-muted-foreground">{label}</dt>
			<dd className="min-w-0 break-words">{children}</dd>
		</>
	);
}

function HistoryList({
	history,
	now,
}: {
	history: InspectorScheduleFire[];
	now: number;
}) {
	return (
		<div className="divide-y rounded-md border">
			{history.map((fire, index) => (
				<div
					key={`${fire.firedAt}:${index}`}
					className="grid grid-cols-[6rem_1fr_auto] items-start gap-3 p-3 text-xs"
				>
					<ResultBadge result={fire.result} />
					<div>
						<div>{formatTimestamp(fire.firedAt)}</div>
						{fire.error && (
							<div className="mt-1 text-muted-foreground">
								{fire.error.code}: {fire.error.message}
							</div>
						)}
					</div>
					<div className="text-right text-muted-foreground">
						{formatDuration((fire.finishedAt ?? now) - fire.firedAt)}
					</div>
				</div>
			))}
		</div>
	);
}

function KindBadge({ kind }: { kind: InspectorSchedule["kind"] }) {
	return (
		<Badge variant="outline" className="capitalize">
			{kind === "at" ? "One-time" : kind}
		</Badge>
	);
}

function ResultBadge({ result }: { result: InspectorScheduleFire["result"] }) {
	return (
		<div className="flex items-center gap-1.5 capitalize">
			<span
				className={cn(
					"size-1.5 rounded-full",
					result === "ok" && "bg-emerald-500",
					result === "error" && "bg-destructive",
					result === "running" && "animate-pulse bg-blue-500",
					result === "skipped" && "bg-amber-500",
				)}
			/>
			{result === "ok" ? "Success" : result}
		</div>
	);
}

function RelativeTime({ timestamp, now }: { timestamp: number; now: number }) {
	const delta = timestamp - now;
	const future = delta >= 0;
	const absolute = Math.abs(delta);
	let value: string;
	if (absolute < 60_000) value = `${Math.max(1, Math.round(absolute / 1_000))}s`;
	else if (absolute < 3_600_000) value = `${Math.round(absolute / 60_000)}m`;
	else if (absolute < 86_400_000) value = `${Math.round(absolute / 3_600_000)}h`;
	else value = `${Math.round(absolute / 86_400_000)}d`;
	return (
		<time dateTime={new Date(timestamp).toISOString()} title={formatTimestamp(timestamp)}>
			{future ? `in ${value}` : `${value} ago`}
		</time>
	);
}

function formatTimestamp(timestamp: number): string {
	return new Intl.DateTimeFormat(undefined, {
		dateStyle: "medium",
		timeStyle: "medium",
	}).format(timestamp);
}
