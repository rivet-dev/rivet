import type { Rivet } from "@rivet-gg/api";
import {
	Button,
	RelativeTime,
	SmallText,
	WithTooltip,
	cn,
} from "@rivet-gg/components";
import { Link } from "@tanstack/react-router";
import { memo } from "react";
import { ActorRegion } from "./actor-region";
import { ActorStatusIndicator } from "./actor-status-indicator";
import { ActorTags } from "./actor-tags";

const BUILT_IN_TAGS = ["name"];

interface ActorsListRowProps
	extends Pick<
		Rivet.actor.Actor,
		"createdAt" | "destroyedAt" | "startedAt" | "tags" | "id" | "region"
	> {
	isCurrent?: boolean;
	projectNameId: string;
	environmentNameId: string;
	className?: string;
}

export const ActorsListRow = memo(
	({
		projectNameId,
		environmentNameId,
		isCurrent,
		createdAt,
		destroyedAt,
		startedAt,
		tags,
		id,
		region,
		...props
	}: ActorsListRowProps) => {
		return (
			<Button
				{...props}
				className={cn(
					"h-auto grid grid-cols-subgrid col-span-full py-4 px-0 group border-l-0 border-r-0 border-t first-of-type:border-t-transparent border-b-transparent last-of-type:border-b-border rounded-none pr-4",
					props.className,
				)}
				variant={isCurrent ? "secondary" : "outline"}
				asChild
			>
				<Link
					to="."
					search={(search) => ({
						...search,
						actorId: id,
						tab: "logs",
					})}
					className="min-w-0 flex-wrap gap-2"
				>
					<div className="w-full flex justify-center">
						<ActorStatusIndicator
							destroyedAt={destroyedAt}
							createdAt={createdAt}
							startedAt={startedAt}
						/>
					</div>
					<SmallText className="font-semibold">
						<ActorRegion
							showLabel="abbreviated"
							projectNameId={projectNameId}
							environmentNameId={environmentNameId}
							regionId={region}
						/>
					</SmallText>
					<SmallText>{id.split("-")[0]}</SmallText>
					<SmallText>
						{(tags as Record<string, string>).name ?? "-"}
					</SmallText>
					<WithTooltip
						trigger={
							<div className="relative overflow-r-gradient">
								<ActorTags
									className="flex-nowrap empty:block overflow-hidden"
									truncate={false}
									tags={tags}
									excludeBuiltIn="builds"
								/>
							</div>
						}
						content={
							<>
								<p className="pb-2 font-bold text-xs">Tags</p>
								<ActorTags
									className="empty:block"
									truncate={false}
									tags={tags}
								/>
							</>
						}
					/>
					<SmallText className="mx-1">
						<WithTooltip
							trigger={
								<RelativeTime time={new Date(createdAt)} />
							}
							content={new Date(createdAt).toLocaleString()}
						/>
					</SmallText>

					<SmallText className="mx-1">
						{destroyedAt ? (
							<WithTooltip
								trigger={
									<RelativeTime
										time={new Date(destroyedAt)}
									/>
								}
								content={new Date(destroyedAt).toLocaleString()}
							/>
						) : (
							<span>-</span>
						)}
					</SmallText>
				</Link>
			</Button>
		);
	},
);
