import type { Rivet } from "@rivet-gg/api";
import { Button, LogsView, WithTooltip } from "@rivet-gg/components";
import { Icon, faSave } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import { differenceInHours } from "date-fns";
import { saveAs } from "file-saver";
import { actorLogsQueryOptions } from "../../queries";

interface ActorLogsTabProps {
	projectNameId: string;
	environmentNameId: string;
	actorId: string;
	logType: Rivet.actor.LogStream;
	createdAt: Rivet.Timestamp;
}

export function ActorLogsTab({
	projectNameId,
	environmentNameId,
	actorId,
	logType,
	createdAt,
}: ActorLogsTabProps) {
	const {
		data: { timestamps, lines },
	} = useSuspenseQuery(
		actorLogsQueryOptions({
			projectNameId,
			environmentNameId,
			actorId,
			stream: logType,
		}),
	);

	const areLogsRetained = differenceInHours(Date.now(), createdAt) < 24 * 3;

	return (
		<LogsView
			timestamps={timestamps}
			lines={lines}
			empty={
				areLogsRetained
					? "No logs available."
					: "Logs are retained for 3 days."
			}
			sidebar={
				lines.length > 0 ? (
					<WithTooltip
						content="Download logs"
						trigger={
							<Button
								variant="outline"
								aria-label="Download logs"
								size="icon"
								onClick={() =>
									saveAs(
										new Blob([lines.join("\n")], {
											type: "text/plain;charset=utf-8",
										}),
										"logs.txt",
									)
								}
							>
								<Icon icon={faSave} />
							</Button>
						}
					/>
				) : null
			}
		/>
	);
}

ActorLogsTab.Skeleton = () => {
	return (
		<div className="px-4 pt-4">
			<LogsView.Skeleton />
		</div>
	);
};
