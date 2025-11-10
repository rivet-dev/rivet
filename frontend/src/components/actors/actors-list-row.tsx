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
								trigger={
									<div className="w-full flex justify-center">
										<QueriedActorStatusIndicator
											actorId={actorId}
										/>
									</div>
								}
								content={
									<QueriedActorStatusLabel
										actorId={actorId}
									/>
								}
							/>
							<div className="min-w-0 flex items-center">
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
			className="text-muted-foreground tabular-nums font-mono-console mr-2 inline-flex my-0 py-0 border-0 h-auto"
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
	const { data: datacenter } = useQuery({
		...useDataProvider().actorDatacenterQueryOptions(actorId),
		enabled: showDatacenter,
	});

	if (!showDatacenter) {
		return <div />;
	}

	if (!datacenter) {
		return <SmallText className="text-foreground mr-2">-</SmallText>;
	}

	return (
		<SmallText className="text-muted-foreground mr-2">
			{datacenter}
		</SmallText>
	);
}

function Tags({ actorId }: { actorId: ActorId }) {
	const { data } = useQuery(useDataProvider().actorKeysQueryOptions(actorId));

	return (
		<SmallText className="text-foreground truncate min-w-0 max-w-full inline-block">
			{data || "-"}
		</SmallText>
	);
}

function Timestamp({ actorId }: { actorId: ActorId }) {
	const { data: { createdAt, destroyedAt } = {} } = useQuery(
		useDataProvider().actorQueryOptions(actorId),
	);

	const ts = destroyedAt || createdAt;

	const timestamp = ts ? new Date(ts) : null;

	return (
		<SmallText className="mx-1 text-right text-muted-foreground">
			{timestamp ? (
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
	return (
		<>
			<div className="size-full items-center justify-center flex">
				<ActorStatusIndicator status="unknown" />
			</div>
			<Skeleton className="h-full w-1/3" />
			<div className="size-full flex justify-end">
				<Skeleton className="h-full w-1/3" />
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
