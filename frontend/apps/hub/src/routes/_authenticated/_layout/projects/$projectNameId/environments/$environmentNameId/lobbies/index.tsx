import { LobbySortSelect } from "@/domains/project/components/matchmaker/lobby-sort-select";
import { ProjectMatchmakerListLobbyPreview } from "@/domains/project/components/matchmaker/matchmaker-list-lobby-preview";
import { useEnvironment } from "@/domains/project/data/environment-context";
import { useProject } from "@/domains/project/data/project-context";
import * as Layout from "@/domains/project/layouts/matchmaker-layout";
import { projectEnvironmentLobbiesLiveQueryOptions } from "@/domains/project/queries";
import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	LiveBadge,
} from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { useMemo } from "react";
import { z } from "zod";

function MatchmakerLobbiesView() {
	const { namespaceId: environmentId } = useEnvironment();
	const { gameId: projectId } = useProject();

	const { sort, lobbyId } = Route.useSearch();

	const navigate = useNavigate();
	const {
		data: { lobbies },
	} = useSuspenseQuery(
		projectEnvironmentLobbiesLiveQueryOptions({ projectId, environmentId }),
	);

	const sorted = useMemo(() => {
		if (!lobbies) {
			return [];
		}

		if (sort === "creation-date-oldest") {
			return lobbies.sort((a, b) => {
				return +a.createTs - +b.createTs;
			});
		}

		if (sort === "status") {
			return lobbies.sort((a, b) => {
				return a.readableStatus.localeCompare(b.readableStatus);
			});
		}

		if (sort === "player-count-biggest") {
			return lobbies.sort((a, b) => {
				return b.totalPlayerCount - a.totalPlayerCount;
			});
		}

		if (sort === "player-count-smallest") {
			return lobbies.sort((a, b) => {
				return a.totalPlayerCount - b.totalPlayerCount;
			});
		}

		return lobbies.sort((a, b) => {
			return +b.createTs - +a.createTs;
		});
	}, [lobbies, sort]);

	return (
		<Card className="h-full max-h-full flex flex-col p-0">
			<CardHeader className="border-b flex flex-row justify-between items-center">
				<CardTitle className="flex gap-4">
					Lobbies
					<LiveBadge />
				</CardTitle>
				<LobbySortSelect
					defaultValue="creation-date-newest"
					value={sort}
					onValueChange={(value) => {
						navigate({
							to: ".",
							search: (prev: Record<string, unknown>) => ({
								...prev,
								sort: value,
							}),
						});
					}}
				/>
			</CardHeader>
			<CardContent className="flex-1 min-h-0 w-full p-0">
				{sorted.length === 0 ? (
					<div className="flex items-center mx-auto flex-col gap-2 my-10">
						<span>No lobbies created.</span>
						<span className="text-xs">
							Run your project client & connect to start a lobby.
						</span>
					</div>
				) : (
					<ProjectMatchmakerListLobbyPreview
						lobbies={sorted}
						projectId={projectId}
						environmentId={environmentId}
						lobbyId={lobbyId}
						isLive
					/>
				)}
			</CardContent>
		</Card>
	);
}

const searchSchema = z.object({
	lobbyId: z.string().optional(),
	sort: z.string().optional(),
});

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/lobbies/",
)({
	validateSearch: zodValidator(searchSchema),
	staticData: {
		layout: "full",
	},
	component: MatchmakerLobbiesView,
	pendingComponent: Layout.Content.Skeleton,
});
