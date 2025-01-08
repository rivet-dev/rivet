import type { Rivet } from "@rivet-gg/api";
import { Button, LogsView, WithTooltip } from "@rivet-gg/components";
import { Icon, faSave } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import {
	projectEnvironmentLogsLobbyLogsQueryOptions,
	useExportLobbyLogsMutation,
} from "../../queries";

interface LobbyLogsProps {
	projectId: string;
	lobbyId: string;
	isLive?: boolean;
	logType: Rivet.cloud.games.LogStream;
}

export function LobbyLogs({
	projectId,
	lobbyId,
	isLive,
	logType,
}: LobbyLogsProps) {
	const {
		data: { timestamps, lines },
	} = useSuspenseQuery(
		projectEnvironmentLogsLobbyLogsQueryOptions(
			{
				projectId,
				lobbyId,
				stream: logType,
			},
			{ refetchInterval: isLive ? 1000 : undefined },
		),
	);

	const { mutate: download, isPending } = useExportLobbyLogsMutation();

	return (
		<LogsView
			timestamps={timestamps}
			lines={lines}
			showTurncatedLogsInfo
			sidebar={
				<WithTooltip
					content="Download logs"
					trigger={
						<Button
							isLoading={isPending}
							variant="outline"
							aria-label="Download logs"
							size="icon"
							onClick={() =>
								download({
									lobbyId,
									projectId,
									stream: logType,
								})
							}
						>
							<Icon icon={faSave} />
						</Button>
					}
				/>
			}
		/>
	);
}

LobbyLogs.Skeleton = () => {
	return (
		<div className="px-4 pt-4">
			<LogsView.Skeleton />
		</div>
	);
};
