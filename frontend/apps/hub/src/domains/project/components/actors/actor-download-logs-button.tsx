import { queryClient } from "@/queries/global";
import { Button, WithTooltip } from "@rivet-gg/components";
import { Icon, faSave } from "@rivet-gg/icons";
import { useMutation } from "@tanstack/react-query";
import saveAs from "file-saver";
import { actorLogsQueryOptions } from "../../queries";
import { useActorDetailsSettings } from "./actor-details-settings";
import { type LogsTypeFilter, filterLogs } from "./actor-logs";

interface ActorDownloadLogsButtonProps {
	actorId: string;
	projectNameId: string;
	environmentNameId: string;
	typeFilter?: LogsTypeFilter;
	filter?: string;
}

export function ActorDownloadLogsButton({
	actorId,
	projectNameId,
	environmentNameId,
	typeFilter,
	filter,
}: ActorDownloadLogsButtonProps) {
	const [settings] = useActorDetailsSettings();
	const { mutate, isPending } = useMutation({
		mutationFn: async () => {
			const logs = await queryClient.fetchQuery(
				actorLogsQueryOptions({
					actorId,
					projectNameId,
					environmentNameId,
					stream: "std_out",
				}),
			);
			const errors = await queryClient.fetchQuery(
				actorLogsQueryOptions({
					actorId,
					projectNameId,
					environmentNameId,
					stream: "std_err",
				}),
			);

			const combined = filterLogs({
				typeFilter: typeFilter ?? "all",
				filter: filter ?? "",
				logs,
				errors,
			});

			const lines = combined.map((log) => {
				const timestamp = new Date(log.timestamp).toISOString();
				if (settings.showTimestmaps) {
					return `[${timestamp}] ${log.message}`;
				}
				return log.message;
			});

			saveAs(
				new Blob([lines.join("\n")], {
					type: "text/plain;charset=utf-8",
				}),
				"logs.txt",
			);
		},
	});

	return (
		<WithTooltip
			content="Download logs"
			trigger={
				<Button
					className="ml-2 place-self-center"
					variant="outline"
					aria-label="Download logs"
					size="icon-sm"
					isLoading={isPending}
					onClick={() => mutate()}
				>
					<Icon icon={faSave} />
				</Button>
			}
		/>
	);
}
