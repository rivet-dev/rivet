import { GetStarted } from "@/components/get-started";
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
	WithTooltip,
} from "@rivet-gg/components";
import { Icon, faActors, faRefresh } from "@rivet-gg/icons";
import { useSuspenseInfiniteQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { zodSearchValidator } from "@tanstack/router-zod-adapter";
import { z } from "zod";

function ProjectActorsRoute() {
	const {
		environment: { nameId: environmentNameId },
		project: { nameId: projectNameId },
	} = Route.useRouteContext();
	const { data, refetch, isRefetching } = useSuspenseInfiniteQuery(
		projectActorsQueryOptions({ projectNameId, environmentNameId }),
	);
	const { actorId } = Route.useSearch();

	if (data.length === 0) {
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
		<Card w="full" h="full" className="flex flex-col">
			<CardHeader className="border-b ">
				<CardTitle className="flex flex-row justify-between items-center">
					Actors
					<Flex gap="2">
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
				/>
			</CardContent>
		</Card>
	);
}

const searchSchema = z.object({
	actorId: z.string().optional(),
	tab: z.string().optional(),
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
