import { Button, LogsView, WithTooltip } from "@rivet-gg/components";
import { Icon, faSave } from "@rivet-gg/icons";
import saveAs from "file-saver";
import type { BackendEvent } from "../../../queries";

interface BackendEventDetailsLogsTabProps
	extends Pick<BackendEvent, "logs" | "logTimestamps"> {}

export function BackendEventDetailsLogsTab({
	logs,
	logTimestamps,
}: BackendEventDetailsLogsTabProps) {
	return (
		<div className="h-full py-4 px-4">
			<LogsView
				timestamps={logTimestamps}
				lines={logs}
				showFollowToggle={false}
				empty={<p>No logs available.</p>}
				sidebar={
					<WithTooltip
						content="Download logs"
						trigger={
							<Button
								variant="outline"
								aria-label="Download logs"
								size="icon"
								onClick={() =>
									saveAs(
										new Blob(
											[
												logs
													.map((line) => line.message)
													.join("\n"),
											],
											{
												type: "text/plain;charset=utf-8",
											},
										),
										"logs.txt",
									)
								}
							>
								<Icon icon={faSave} />
							</Button>
						}
					/>
				}
			/>
		</div>
	);
}
