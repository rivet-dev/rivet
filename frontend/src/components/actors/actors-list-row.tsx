import { useQuery } from "@tanstack/react-query";
import { Link, useSearch } from "@tanstack/react-router";
import { motion } from "framer-motion";
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
import { useActorsFilters } from "./actor-filters-context";
import {
	ActorStatusIndicator,
	QueriedActorStatusIndicator,
} from "./actor-status-indicator";
import { QueriedActorStatusLabel } from "./actor-status-label";
import { useManager } from "./manager-context";
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
							<div>
								<Id actorId={actorId} />
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
	const { pick } = useActorsFilters();
	const showIds = useSearch({
		strict: false,
		select: (search) => pick(search).showIds?.value?.includes("1"),
	});

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

function Tags({ actorId }: { actorId: ActorId }) {
	const { data } = useQuery(useManager().actorKeysQueryOptions(actorId));

	return <SmallText className="text-foreground">{data || "-"}</SmallText>;
}

function Timestamp({ actorId }: { actorId: ActorId }) {
	const { data: { createdAt, destroyedAt } = {} } = useQuery(
		useManager().actorQueryOptions(actorId),
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
