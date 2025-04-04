import { Button, DocsSheet, ScrollArea, SmallText } from "@rivet-gg/components";
import { ActorsFiltersButton } from "./actors-filters-button";
import { ActorsListRow } from "./actors-list-row";
import { CreateActorButton } from "./create-actor-button";
import { GoToActorButton } from "./go-to-actor-button";
import { useAtomValue, useSetAtom } from "jotai";
import {
	actorFiltersAtom,
	actorFiltersCountAtom,
	actorsAtomsAtom,
	actorsPaginationAtom,
	filteredActorsCountAtom,
} from "./actor-context";
import { faReact, faRust, faTs, Icon } from "@rivet-gg/icons";

export function ActorsList() {
	return (
		<ScrollArea className="w-full">
			<div className="grid grid-cols-[2rem_min-content_min-content_minmax(min-content,1fr)_minmax(1.5rem,3fr)_minmax(min-content,1fr)_minmax(min-content,1fr)] items-center justify-center gap-x-4 w-full min-w-[450px]">
				<div className="grid grid-cols-subgrid col-span-full sticky top-0 border-b z-[1] bg-card">
					<div className="col-span-full border-b justify-between flex  p-1 py-2 gap-1">
						<ActorsFiltersButton />
						<div className="flex gap-1">
							<GoToActorButton />
							<CreateActorButton />
						</div>
					</div>
					<div className="grid grid-cols-subgrid col-span-full  font-semibold text-sm px-1 pr-4">
						<div />
						<div className="pb-3 pt-3">Region</div>
						<div className="pb-3 pt-3">ID</div>
						<div className="pb-3 pt-3">Tags</div>
						<div className="pb-3 pt-3">Created</div>
						<div className="pb-3 pt-3">Destroyed</div>
					</div>
				</div>
				<List />
				<Pagination />
			</div>
		</ScrollArea>
	);
}

function List() {
	const actors = useAtomValue(actorsAtomsAtom);
	return (
		<>
			{actors.map((actor) => (
				<ActorsListRow key={`${actor}`} actor={actor} />
			))}
		</>
	);
}

function Pagination() {
	const { hasNextPage, isFetchingNextPage, fetchNextPage } =
		useAtomValue(actorsPaginationAtom);

	if (hasNextPage) {
		return (
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
		);
	}

	return <EmptyState />;
}

function EmptyState() {
	const count = useAtomValue(filteredActorsCountAtom);
	const filtersCount = useAtomValue(actorFiltersCountAtom);
	const setFilters = useSetAtom(actorFiltersAtom);
	return (
		<div className=" col-span-full my-4 flex flex-col items-center gap-2 justify-center">
			{count === 0 ? (
				filtersCount === 0 ? (
					<div className="gap-2 flex flex-col items-center justify-center">
						<SmallText className="text-center">
							No actors found.
						</SmallText>
						<div className="mt-4 flex flex-col gap-2 items-center justify-center">
							<CreateActorButton variant="outline" />{" "}
							<SmallText className="mt-2">
								Use one of the quick start guides to get
								started.
							</SmallText>
							<div className="flex gap-2">
								<DocsSheet
									path="docs/quickstart/react"
									title="React Quick Start"
								>
									<Button
										variant="outline"
										size="sm"
										startIcon={<Icon icon={faReact} />}
									>
										React
									</Button>
								</DocsSheet>
								<DocsSheet
									path="docs/quickstart/typescript"
									title="TypeScript Quick Start"
								>
									<Button
										variant="outline"
										size="sm"
										startIcon={<Icon icon={faTs} />}
									>
										TypeScript
									</Button>
								</DocsSheet>
								<DocsSheet
									path="docs/quickstart/typescript"
									title="Rust Quick Start"
								>
									<Button
										variant="outline"
										size="sm"
										startIcon={<Icon icon={faRust} />}
									>
										Rust
									</Button>
								</DocsSheet>
							</div>
						</div>
					</div>
				) : (
					<>
						<SmallText className="text-muted-foreground text-center">
							No actors match the filters.
						</SmallText>
						<Button
							variant="outline"
							mx="4"
							onClick={() =>
								setFilters({ showDestroyed: true, tags: {} })
							}
						>
							Clear filters
						</Button>
					</>
				)
			) : (
				<SmallText className="text-muted-foreground text-center">
					No more actors to load.
				</SmallText>
			)}
		</div>
	);
}
