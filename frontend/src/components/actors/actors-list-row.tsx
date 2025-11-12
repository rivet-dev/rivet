import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { memo, useState } from "react";
import {
	Button,
	cn,
	DiscreteCopyButton,
	RelativeTime,
	Skeleton,
	SmallText,
	WithTooltip,
} from "@/components";
import { VisibilitySensor } from "../visibility-sensor";
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
	isCurrent?: boolean;
}

export const ActorsListRow = memo(
	({ className, actorId, isCurrent }: ActorsListRowProps) => {
		const [isVisible, setIsVisible] = useState(false);

		return (
			<Button
				className={cn(
					"h-auto grid grid-cols-subgrid col-span-full py-4 px-0 group border-l-0 border-r-0 border-t first-of-type:border-t-transparent border-b-transparent last-of-type:border-b-border rounded-none pr-4 min-h-[56px]",
					className,
				)}
				variant={isCurrent ? "secondary" : "outline"}
				asChild
			>
				<Link
					to="."
					search={(search: Record<string, unknown>) => ({
						...search,
						actorId,
					})}
					className="min-w-0 flex-wrap gap-2 relative"
				>
					{isVisible ? (
						<>
							<WithTooltip
								delayDuration={0}
								trigger={
									<div className="w-full flex justify-center">
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
							<div className="min-w-0 flex items-center gap-1">
								<Id actorId={actorId} />
								<Datacenter actorId={actorId} />
								<Tags actorId={actorId} />
							</div>

							<Timestamp actorId={actorId} />
						</>
					) : (
						<SkeletonContent />
					)}
					<VisibilitySensor
						onToggle={setIsVisible}
						className="absolute"
					/>
				</Link>
			</Button>
		);
	},
);

function Id({ actorId }: { actorId: ActorId }) {
	const showIds = useFiltersValue().showIds?.value.includes("1");

	if (!showIds) {
		return <div />;
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
		return <div />;
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
		<SmallText className="text-foreground truncate min-w-0 max-w-full inline-block">
			{isLoading ? <Skeleton className="h-5 w-10" /> : data || "-"}
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
		<SmallText className="text-right text-muted-foreground flex justify-end">
			{isLoading ? (
				<Skeleton className="h-5 w-10" />
			) : timestamp ? (
				<WithTooltip
					trigger={<RelativeTime time={timestamp} />}
					content={timestamp.toLocaleString()}
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
			<div className="size-full items-center justify-center flex">
				<ActorStatusIndicator status="unknown" />
			</div>
			<div className="min-w-0 flex items-center gap-1">
				{showIds ? <Skeleton className="h-5 w-10" /> : <div />}
				<Skeleton className="h-5 w-10" />
				<Skeleton className="h-5 w-10" />
			</div>
			<div className="size-full flex justify-end">
				<Skeleton className="h-5 w-10" />
			</div>
		</>
	);
}

export function ActorsListRowSkeleton() {
	return (
		<div className="border-b gap-1.5 py-4 pr-4 h-[56px] grid grid-cols-subgrid items-center col-span-full relative">
			<SkeletonContent />
		</div>
	);
}
