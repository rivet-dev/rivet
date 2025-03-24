import { Button, ScrollArea, SmallText } from "@rivet-gg/components";
import { useSuspenseInfiniteQuery } from "@tanstack/react-query";
import { projectActorsQueryOptions } from "../../queries";
import { ActorsFiltersButton } from "./actors-filters-button";
import { ActorsListRow } from "./actors-list-row";
import { CreateActorButton } from "./create-actor-button";
import { GoToActorButton } from "./go-to-actor-button";

interface ActorsListProps {
	projectNameId: string;
	environmentNameId: string;
	actorId: string | undefined;
	tags: Record<string, string>;
	showDestroyed: boolean;
}

export function ActorsList({
	projectNameId,
	environmentNameId,
	actorId,
	tags,
	showDestroyed,
}: ActorsListProps) {
	const { data, hasNextPage, isFetchingNextPage, fetchNextPage } =
		useSuspenseInfiniteQuery(
			projectActorsQueryOptions({
				projectNameId,
				environmentNameId,
				tags,
				includeDestroyed: showDestroyed,
			}),
		);
	return (
		<ScrollArea className="w-full">
			<div className="grid grid-cols-[2rem_min-content_min-content_minmax(min-content,1fr)_minmax(1.5rem,3fr)_minmax(min-content,1fr)_minmax(min-content,1fr)] items-center justify-center gap-x-4 w-full min-w-[450px]">
				<div className="grid grid-cols-subgrid col-span-full sticky top-0 border-b z-[1] bg-card">
					<div className="col-span-full border-b justify-between flex  p-1 py-2 gap-1">
						<ActorsFiltersButton
							showDestroyed={showDestroyed}
							tags={tags}
						/>
						<div className="flex gap-1">
							<GoToActorButton />
							<CreateActorButton />
						</div>
					</div>
					<div className="grid grid-cols-subgrid col-span-full  font-semibold text-sm px-1 pr-4">
						<div />
						<div className="pb-3 pt-3">Region</div>
						<div className="pb-3 pt-3">ID</div>
						<div className="pb-3 pt-3">Name</div>
						<div className="pb-3 pt-3">Tags</div>
						<div className="pb-3 pt-3">Created</div>
						<div className="pb-3 pt-3">Destroyed</div>
					</div>
				</div>
				<>
					{data.map((actor) => (
						<ActorsListRow
							key={actor.id}
							projectNameId={projectNameId}
							environmentNameId={environmentNameId}
							isCurrent={actor.id === actorId}
							createdAt={actor.createdAt}
							destroyedAt={actor.destroyedAt}
							startedAt={actor.startedAt}
							tags={actor.tags}
							id={actor.id}
							region={actor.region}
						/>
					))}
					{hasNextPage ? (
						<div className="col-span-full flex w-full justify-center py-4">
							<Button
								variant="outline"
								mx="4"
								isLoading={isFetchingNextPage}
								onClick={() => fetchNextPage()}
							>
								Load more
							</Button>
						</div>
					) : (
						<div className=" col-span-full my-4 flex flex-col items-center gap-2 justify-center">
							<SmallText className="text-muted-foreground text-center">
								{data.length === 0
									? "No actors found."
									: "No more actors to load."}
							</SmallText>
							<CreateActorButton variant="outline" />
						</div>
					)}
				</>
			</div>
		</ScrollArea>
	);
}
