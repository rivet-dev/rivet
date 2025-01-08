import {
	Badge,
	Button,
	Flex,
	SmallText,
	Uptime,
	WithTooltip,
} from "@rivet-gg/components";
import { Icon, faUsers } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import {
	type LiveLobbyLogs,
	type LobbySummary,
	projectEnvironmentLobbyQueryOptions,
} from "../../queries";
import { LobbyRegion } from "./lobby-region";
import { LobbyStatusBadge } from "./lobby-status";

type ProjectMatchmakerListLobbyButtonProps = (LobbySummary | LiveLobbyLogs) & {
	projectId: string;
	isActive?: boolean;
};
export function ProjectMatchmakerListLobbyButton({
	isActive,
	lobbyId,
	regionId,
	projectId,
	lobbyGroupNameId,
	readableStatus,
	createTs,
	...props
}: ProjectMatchmakerListLobbyButtonProps) {
	return (
		<Button
			key={lobbyId}
			variant={isActive ? "secondary" : "outline"}
			asChild
		>
			<Link
				to="."
				search={(old) => ({ ...old, lobbyId: lobbyId })}
				className="truncate min-w-0"
			>
				<Flex gap="2" items="center" w="full">
					<Badge variant="outline">
						<LobbyRegion
							projectId={projectId}
							regionId={regionId}
						/>
					</Badge>
					<LobbyStatusBadge status={readableStatus} />
					<span className="flex-1 text-left">{lobbyGroupNameId}</span>
					{"totalPlayerCount" in props ? (
						<Badge variant="outline">
							<Flex gap="2">
								<span>
									{props.totalPlayerCount} /{" "}
									{props.maxPlayersNormal}
								</span>

								<Icon className="size-4" icon={faUsers} />
							</Flex>
						</Badge>
					) : null}
					<SmallText>
						{["closed", "failed"].includes(readableStatus) ? (
							createTs.toLocaleString()
						) : (
							<WithTooltip
								trigger={
									<SmallText>
										<Uptime createTs={createTs} />
									</SmallText>
								}
								content={createTs.toLocaleString()}
							/>
						)}
					</SmallText>
				</Flex>
			</Link>
		</Button>
	);
}

interface ProjectMatchmakerListLobbyPrefilledButtonProps {
	projectId: string;
	environmentId: string;
	lobbyId: string;
	isActive?: boolean;
}

export function ProjectMatchmakerListLobbyPrefilledButton({
	projectId,
	environmentId,
	lobbyId,
	isActive,
}: ProjectMatchmakerListLobbyPrefilledButtonProps) {
	const {
		data: { lobby },
	} = useSuspenseQuery(
		projectEnvironmentLobbyQueryOptions({
			projectId,
			environmentId,
			lobbyId,
		}),
	);

	return (
		<ProjectMatchmakerListLobbyButton
			isActive={isActive}
			projectId={projectId}
			{...lobby}
		/>
	);
}
