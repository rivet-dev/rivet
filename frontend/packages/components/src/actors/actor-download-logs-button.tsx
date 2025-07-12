import { Button, WithTooltip } from "@rivet-gg/components";
import { Icon, faSave } from "@rivet-gg/icons";
import { useAtomValue } from "jotai";
import type { ActorAtom } from "./actor-context";
import type { LogsTypeFilter } from "./actor-logs";

interface ActorDownloadLogsButtonProps {
	actor: ActorAtom;
	typeFilter?: LogsTypeFilter;
	filter?: string;
	onExportLogs?: (
		actorId: string,
		typeFilter?: string,
		filter?: string,
	) => Promise<void>;
	isExporting?: boolean;
}

export function ActorDownloadLogsButton({
	actor,
	typeFilter,
	filter,
	onExportLogs,
	isExporting = false,
}: ActorDownloadLogsButtonProps) {
	const actorData = useAtomValue(actor);

	const handleDownload = async () => {
		if (!onExportLogs) {
			console.warn("No export handler provided");
			return;
		}

		try {
			await onExportLogs(actorData.id, typeFilter, filter);
		} catch (error) {
			console.error("Failed to export logs:", error);
		}
	};

	return (
		<WithTooltip
			content="Export logs"
			trigger={
				<Button
					className="ml-2 place-self-center"
					variant="outline"
					aria-label="Export logs"
					size="icon-sm"
					onClick={handleDownload}
					disabled={isExporting || !onExportLogs}
				>
					<Icon
						icon={faSave}
						className={isExporting ? "animate-pulse" : ""}
					/>
				</Button>
			}
		/>
	);
}
