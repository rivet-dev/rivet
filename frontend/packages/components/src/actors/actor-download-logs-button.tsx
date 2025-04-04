import { Button, WithTooltip } from "@rivet-gg/components";
import { Icon, faSave } from "@rivet-gg/icons";
import saveAs from "file-saver";
import {
	type Settings,
	useActorDetailsSettings,
} from "./actor-details-settings";
import { type LogsTypeFilter, filterLogs } from "./actor-logs";
import type { ActorAtom, LogsAtom } from "./actor-context";
import { selectAtom } from "jotai/utils";
import { type Atom, atom, useAtom } from "jotai";

const downloadLogsAtom = atom(
	null,
	(
		get,
		_set,
		{
			typeFilter,
			filter,
			settings,
			logs: logsAtom,
		}: {
			typeFilter?: LogsTypeFilter;
			filter?: string;
			settings: Settings;
			logs: Atom<LogsAtom>;
		},
	) => {
		const { logs, errors } = get(get(logsAtom));

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
);

interface ActorDownloadLogsButtonProps {
	actor: ActorAtom;
	typeFilter?: LogsTypeFilter;
	filter?: string;
}

export function ActorDownloadLogsButton({
	actor,
	typeFilter,
	filter,
}: ActorDownloadLogsButtonProps) {
	const [settings] = useActorDetailsSettings();

	const [, downloadLogs] = useAtom(downloadLogsAtom);

	return (
		<WithTooltip
			content="Download logs"
			trigger={
				<Button
					className="ml-2 place-self-center"
					variant="outline"
					aria-label="Download logs"
					size="icon-sm"
					onClick={() =>
						downloadLogs({
							typeFilter,
							filter,
							settings,
							logs: selectAtom(actor, (a) => a.logs),
						})
					}
				>
					<Icon icon={faSave} />
				</Button>
			}
		/>
	);
}
