import {
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
	useBreakpoint,
} from "@rivet-gg/components";
import { type ReactNode, Suspense } from "react";
import type { LiveLobbyLogs, LobbySummary } from "../../queries";
import { ProjectMatchmakerListLobbyPrefilledButton } from "./matchmaker-list-lobby-button";
import { ProjectMatchmakerLobbyListPanel } from "./matchmaker-list-lobby-panel";
import { ProjectMatchmakerLobbyDetailsPanel } from "./matchmaker-lobby-details-panel";

interface ProjectMatchmakerListLobbyPreviewProps {
	lobbies: (LobbySummary | LiveLobbyLogs)[];
	lobbyId?: string;
	projectId: string;
	environmentId: string;
	isLive?: boolean;
	sort?: ReactNode;
}

export function ProjectMatchmakerListLobbyPreview({
	lobbies,
	lobbyId,
	projectId,
	environmentId,
	isLive,
}: ProjectMatchmakerListLobbyPreviewProps) {
	const isMd = useBreakpoint("md");
	const doesLobbyExist =
		lobbies.find((lobby) => lobby.lobbyId === lobbyId) !== undefined;

	return (
		<ResizablePanelGroup
			className="min-w-0 w-full h-full max-h-full"
			autoSaveId="rivet-project-backend-logs"
			direction={isMd ? "horizontal" : "vertical"}
		>
			<ResizablePanel minSize={25} maxSize={75}>
				<div className="h-full max-h-full overflow-hidden w-full min-w-0">
					<ProjectMatchmakerLobbyListPanel
						projectId={projectId}
						lobbies={lobbies}
						lobbyId={lobbyId}
					>
						{!doesLobbyExist && lobbyId ? (
							<ProjectMatchmakerListLobbyPrefilledButton
								isActive
								projectId={projectId}
								environmentId={environmentId}
								lobbyId={lobbyId}
							/>
						) : null}
					</ProjectMatchmakerLobbyListPanel>
				</div>
			</ResizablePanel>
			<ResizableHandle withHandle />
			<ResizablePanel minSize={25} maxSize={75}>
				<div className="h-full max-h-full overflow-hidden w-full flex flex-col">
					<Suspense
						fallback={
							<ProjectMatchmakerLobbyDetailsPanel.Skeleton />
						}
					>
						<ProjectMatchmakerLobbyDetailsPanel
							lobbyId={lobbyId}
							projectId={projectId}
							environmentId={environmentId}
							isLive={isLive && doesLobbyExist}
						/>
					</Suspense>
				</div>
			</ResizablePanel>
		</ResizablePanelGroup>
	);
}
