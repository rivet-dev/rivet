import { faSave, Icon } from "@rivet-gg/icons";
import { Button, WithTooltip } from "@/components";
import type { LogsTypeFilter } from "./actor-logs";
import type { ActorId } from "./queries";

interface ActorDownloadLogsButtonProps {
	actorId: ActorId;
	typeFilter?: LogsTypeFilter;
	filter?: string;
	onExportLogs?: (
		actorId: string,
		typeFilter?: string,
		filter?: string,
	) => Promise<void>;
	isExporting?: boolean;
}

export function ActorDownloadLogsButton(_props: ActorDownloadLogsButtonProps) {
	return (
		<WithTooltip
			content="Export logs"
			trigger={
				<Button
					className="ml-2 place-self-center"
					variant="outline"
					aria-label="Export logs"
					size="icon-sm"
				>
					<Icon icon={faSave} />
				</Button>
			}
		/>
	);
}
