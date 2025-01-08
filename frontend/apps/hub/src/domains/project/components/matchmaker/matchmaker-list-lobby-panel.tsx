import { Flex, ScrollArea } from "@rivet-gg/components";
import type { ReactNode } from "react";
import type { LiveLobbyLogs, LobbySummary } from "../../queries";
import { ProjectMatchmakerListLobbyButton } from "./matchmaker-list-lobby-button";

interface ProjectMatchmakerLobbyListPanelProps {
	projectId: string;
	lobbies: (LobbySummary | LiveLobbyLogs)[];
	lobbyId?: string;
	children?: ReactNode;
}

export function ProjectMatchmakerLobbyListPanel({
	lobbies,
	projectId,
	lobbyId,
	children,
}: ProjectMatchmakerLobbyListPanelProps) {
	return (
		<ScrollArea className="overflow-auto h-full truncate min-w-0">
			<Flex
				direction="col"
				gap="2"
				my="4"
				mx="4"
				className="truncate min-w-0"
			>
				<>
					{children}
					{lobbies.map((lobby) => (
						<ProjectMatchmakerListLobbyButton
							key={lobby.lobbyId}
							projectId={projectId}
							isActive={lobby.lobbyId === lobbyId}
							{...lobby}
						/>
					))}
				</>
			</Flex>
		</ScrollArea>
	);
}
