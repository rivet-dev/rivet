import { faCopy, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { memo } from "react";
import {
	Button,
	CopyTrigger,
	cn,
	DiscreteCopyButton,
	RelativeTime,
	Skeleton,
	SmallText,
	WithTooltip,
} from "@/components";
import { useFiltersValue } from "./actor-filters-context";
import { ActorRegion } from "./actor-region";
import {
	ActorStatusIndicator,
	QueriedActorStatusIndicator,
} from "./actor-status-indicator";
import { QueriedActorStatusLabel } from "./actor-status-label";
import { useDataProvider } from "./data-provider";
import type { ActorId } from "./queries";

interface ActorsListRowProps {
	className?: string;
	actorId: ActorId;
	actorKey?: string;
	isCurrent?: boolean;
}

export const ActorsListRow = memo(
	({ className, actorId, actorKey, isCurrent }: ActorsListRowProps) => {
		return (
			<Button
				className={cn(
					"h-[56px] flex items-center w-full group border-l-0 border-r-0 border-t-0 border-b rounded-none pl-2 pr-4",
					className,
				)}
				variant={isCurrent ? "secondary" : "outline"}
				asChild
			>
				<Link
					to="."
					search={(search: Record<string, unknown>) => ({
						...search,
						...(actorKey ? { actorKey } : { actorId }),
					})}
					className="flex items-center gap-2 w-full min-w-0"
				>
					<WithTooltip
						delayDuration={0}
						trigger={
							<div className="w-6 flex-none flex justify-center">
								<QueriedActorStatusIndicator
									actorId={actorId}
								/>
							</div>
						}
						content={
							<div className="flex flex-col">
								<QueriedActorStatusLabel
									actorId={actorId}
									showAdditionalInfo
								/>
							</div>
						}
					/>
					<div className="flex-1 min-w-0 flex items-center gap-1">
						<Id actorId={actorId} />
						<Datacenter actorId={actorId} />
						<Tags actorId={actorId} />
					</div>
					<Timestamp actorId={actorId} />
				</Link>
			</Button>
		);
	},
);

function Id({ actorId }: { actorId: ActorId }) {
	const showIds = useFiltersValue().showIds?.value.includes("1");

	if (!showIds) {
		return null;
	}

	return (
		<SmallText
			className="text-muted-foreground tabular-nums font-mono-console inline-flex my-0 py-0 border-0 h-auto"
			asChild
		>
			<DiscreteCopyButton value={actorId} size="xs">
				{actorId.includes("-")
					? actorId.split("-")[0]
					: actorId.substring(0, 8)}
			</DiscreteCopyButton>
		</SmallText>
	);
}

function Datacenter({ actorId }: { actorId: ActorId }) {
	const showDatacenter =
		useFiltersValue().showDatacenter?.value.includes("1");
	const { data: datacenter, isLoading } = useQuery({
		...useDataProvider().actorDatacenterQueryOptions(actorId),
		enabled: showDatacenter,
	});

	if (!showDatacenter) {
		return null;
	}

	return (
		<SmallText className="text-foreground">
			{isLoading ? (
				<Skeleton className=" h-5 w-10" />
			) : datacenter ? (
				<ActorRegion regionId={datacenter} />
			) : (
				"-"
			)}
		</SmallText>
	);
}

function Tags({ actorId }: { actorId: ActorId }) {
	const { data, isLoading } = useQuery(
		useDataProvider().actorKeysQueryOptions(actorId),
	);

	return (
		<SmallText className="text-foreground truncate min-w-0 max-w-full inline-flex items-center gap-0.5 group">
			{isLoading ? <Skeleton className="h-5 w-10" /> : data || "-"}
			<CopyTrigger value={actorId}>
				<Button
					variant="ghost"
					size="icon-xs"
					className="group-hover:opacity-100 opacity-0 transition-opacity text-sm"
				>
					<Icon icon={faCopy} />
				</Button>
			</CopyTrigger>
		</SmallText>
	);
}

function Timestamp({ actorId }: { actorId: ActorId }) {
	const { data: { createTs, destroyTs } = {}, isLoading } = useQuery(
		useDataProvider().actorQueryOptions(actorId),
	);

	const ts = destroyTs || createTs;

	const timestamp = ts ? new Date(ts) : null;

	return (
		<SmallText className="hidden @xs/main:flex text-right text-muted-foreground justify-end">
			{isLoading ? (
				<Skeleton className="h-5 w-10" />
			) : timestamp ? (
				<WithTooltip
					trigger={<RelativeTime time={timestamp} />}
					content={`Created at ${timestamp.toLocaleString()}`}
				/>
			) : (
				<span>-</span>
			)}
		</SmallText>
	);
}

function SkeletonContent() {
	const showIds = useFiltersValue().showIds?.value.includes("1");

	return (
		<>
			<div className="w-6 flex-none flex items-center justify-center">
				<ActorStatusIndicator status="unknown" />
			</div>
			<div className="flex-1 min-w-0 flex items-center gap-1">
				{showIds ? <Skeleton className="h-5 w-10" /> : <div />}
				<Skeleton className="h-5 w-10" />
				<Skeleton className="h-5 w-10" />
			</div>
			<div className="hidden @xs/main:flex justify-end">
				<Skeleton className="h-5 w-10" />
			</div>
		</>
	);
}

export function ActorsListRowSkeleton() {
	return (
		<div className="border-b flex items-center gap-2 pl-2 pr-4 h-[56px] relative">
			<SkeletonContent />
		</div>
	);
}
