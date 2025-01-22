import { GetStarted } from "@/components/get-started";
import { ActorsFiltersSheet } from "@/domains/project/components/actors/actors-filters-sheet";
import { ActorsListPreview } from "@/domains/project/components/actors/actors-list-preview";
import * as Layout from "@/domains/project/layouts/servers-layout";
import { projectActorsQueryOptions } from "@/domains/project/queries";
import {
	Button,
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	Flex,
	Ping,
	WithTooltip,
} from "@rivet-gg/components";
import { Icon, faActors, faFilter, faRefresh } from "@rivet-gg/icons";
import { useSuspenseInfiniteQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { zodSearchValidator } from "@tanstack/router-zod-adapter";
import { z } from "zod";

function ProjectActorsRoute() {
	const {
		environment: { nameId: environmentNameId, namespaceId: environmentId },
		project: { nameId: projectNameId, gameId: projectId },
	} = Route.useRouteContext();
	const { actorId, tags, showDestroyed } = Route.useSearch();
	const tagsRecord = Object.fromEntries(tags || []);
	const { data, refetch, isRefetching } = useSuspenseInfiniteQuery(
		projectActorsQueryOptions({
			projectNameId,
			environmentNameId,
			tags: tagsRecord,
			includeDestroyed: showDestroyed,
		}),
	);
	const navigate = Route.useNavigate();

	if (data.length === 0 && !tags && showDestroyed === undefined) {
		return (
			<div className="w-full h-full flex flex-col justify-center">
				<div className="flex flex-col justify-center my-8">
					<Icon icon={faActors} className="text-6xl mx-auto my-4" />
					<h3 className="text-center font-bold text-xl max-w-md mb-2 mx-auto">
						Deploy your first Actor
					</h3>
					<p className="text-center text-muted-foreground max-w-sm mx-auto">
						Install Rivet to get started or use an existing template
						to get started.
					</p>
				</div>
				<GetStarted />
			</div>
		);
	}

	return (
		<Card
			w="full"
			// 100vh - header - page padding
			className="flex flex-col h-[calc(100vh-6.5rem-2rem)]"
		>
			<CardHeader className="border-b ">
				<CardTitle className="flex flex-row justify-between items-center">
					Actors
					<Flex gap="2">
						<WithTooltip
							content="Filters"
							trigger={
								<ActorsFiltersSheet
									title="Filters"
									projectId={projectId}
									environmentId={environmentId}
									tags={tagsRecord}
									showDestroyed={showDestroyed || false}
									onFiltersSubmitted={(values) => {
										return navigate({
											search: {
												showDestroyed:
													values.showDestroyed,
												tags: Object.entries(
													values.tags,
												).map(
													([key, value]) =>
														[key, value] as [
															string,
															string,
														],
												),
											},
										});
									}}
								>
									<Button
										size="icon"
										isLoading={isRefetching}
										variant="outline"
									>
										<div className="relative">
											{(tags?.length || 0) > 0 &&
											showDestroyed !== undefined ? (
												<Ping
													variant="primary"
													className="bottom-0 -right-2 top-auto"
												/>
											) : null}
											<Icon icon={faFilter} />
										</div>
									</Button>
								</ActorsFiltersSheet>
							}
						/>
						<WithTooltip
							content="Refresh"
							trigger={
								<Button
									size="icon"
									isLoading={isRefetching}
									variant="outline"
									onClick={() => refetch()}
								>
									<Icon icon={faRefresh} />
								</Button>
							}
						/>
					</Flex>
				</CardTitle>
			</CardHeader>
			<CardContent className="flex-1 min-h-0 w-full p-0">
				<ActorsListPreview
					projectNameId={projectNameId}
					environmentNameId={environmentNameId}
					actorId={actorId}
					tags={tagsRecord}
					showDestroyed={showDestroyed || false}
				/>
			</CardContent>
		</Card>
	);
}

const searchSchema = z.object({
	actorId: z.string().optional(),
	tab: z.string().optional(),

	tags: z.array(z.tuple([z.string(), z.string()])).optional(),
	showDestroyed: z.boolean().optional(),
});

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/actors",
)({
	validateSearch: zodSearchValidator(searchSchema),
	staticData: {
		layout: "full",
	},
	component: ProjectActorsRoute,
	pendingComponent: Layout.Content.Skeleton,
});
