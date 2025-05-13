import {
	Button,
	RelativeTime,
	SmallText,
	WithTooltip,
	cn,
	toRecord,
} from "@rivet-gg/components";
import { Link } from "@tanstack/react-router";
import { memo } from "react";
import { ActorRegion } from "./actor-region";
import { AtomizedActorStatusIndicator } from "./actor-status-indicator";
import { ActorTags } from "./actor-tags";
import {
	isCurrentActorAtom,
	type Actor,
	type ActorAtom,
} from "./actor-context";
import { useAtomValue } from "jotai";
import { selectAtom } from "jotai/utils";

interface ActorsListRowProps {
	className?: string;
	actor: ActorAtom;
}

const selector = (actor: Actor) => actor.id;

export const ActorsListRow = memo(
	({ className, actor }: ActorsListRowProps) => {
		const id = useAtomValue(selectAtom(actor, selector));
		const isCurrent = useAtomValue(isCurrentActorAtom(actor));

		return (
			<Button
				className={cn(
					"h-auto grid grid-cols-subgrid col-span-full py-4 px-0 group border-l-0 border-r-0 border-t first-of-type:border-t-transparent border-b-transparent last-of-type:border-b-border rounded-none pr-4",
					className,
				)}
				variant={isCurrent ? "secondary" : "outline"}
				asChild
			>
				<Link
					to="."
					search={(search: Record<string, unknown>) => ({
						...search,
						actorId: id,
					})}
					className="min-w-0 flex-wrap gap-2"
				>
					<div className="w-full flex justify-center">
						<AtomizedActorStatusIndicator actor={actor} />
					</div>
					<Region actor={actor} />
					<SmallText>{id.split("-")[0]}</SmallText>
					<Tags actor={actor} />

					<CreatedAt actor={actor} />
					<DestroyedAt actor={actor} />
				</Link>
			</Button>
		);
	},
);

const regionSelector = (actor: Actor) => actor.region;

function Region({ actor }: { actor: ActorAtom }) {
	const regionId = useAtomValue(selectAtom(actor, regionSelector));

	return <ActorRegion regionId={regionId} showLabel="abbreviated" />;
}

const tagsSelector = (actor: Actor) => toRecord(actor.tags);

function Tags({ actor }: { actor: ActorAtom }) {
	const tags = useAtomValue(selectAtom(actor, tagsSelector));

	return (
		<WithTooltip
			trigger={
				<div className="relative overflow-r-gradient">
					<ActorTags
						className="flex-nowrap empty:block overflow-hidden"
						truncate={false}
						tags={tags}
						excludeBuiltIn="actors"
					/>
				</div>
			}
			content={
				<>
					<p className="pb-2 font-bold text-xs">Tags</p>
					<ActorTags
						excludeBuiltIn="actors"
						className="empty:block"
						truncate={false}
						tags={tags}
					/>
				</>
			}
		/>
	);
}

const createdAtSelector = (actor: Actor) => actor.createdAt;

function CreatedAt({ actor }: { actor: ActorAtom }) {
	const createdAt = useAtomValue(selectAtom(actor, createdAtSelector));

	return (
		<SmallText className="mx-1">
			{createdAt ? (
				<WithTooltip
					trigger={<RelativeTime time={createdAt} />}
					content={createdAt.toLocaleString()}
				/>
			) : (
				<span>-</span>
			)}
		</SmallText>
	);
}

const destroyedAtSelector = (actor: Actor) => actor.destroyedAt;
function DestroyedAt({ actor }: { actor: ActorAtom }) {
	const destroyedAt = useAtomValue(selectAtom(actor, destroyedAtSelector));

	return (
		<SmallText className="mx-1">
			{destroyedAt ? (
				<WithTooltip
					trigger={<RelativeTime time={new Date(destroyedAt)} />}
					content={new Date(destroyedAt).toLocaleString()}
				/>
			) : (
				<span>-</span>
			)}
		</SmallText>
	);
}
