import { faSave, Icon } from "@rivet-gg/icons";
import saveAs from "file-saver";
import { type Atom, atom, useAtom } from "jotai";
import { selectAtom } from "jotai/utils";
import { Button, WithTooltip } from "@/components";
import type { ActorAtom, LogsAtom } from "./actor-context";
import {
	type Settings,
	useActorDetailsSettings,
} from "./actor-details-settings";
import { filterLogs, type LogsTypeFilter } from "./actor-logs";
import type { ActorId } from "./queries";

// const downloadLogsAtom = atom(
// 	null,
// 	async (
// 		get,
// 		_set,
// 		{
// 			actorId,
// 			typeFilter,
// 			filter,
// 		}: {
// 			actorId: string;
// 			typeFilter?: LogsTypeFilter;
// 			filter?: string;
// 		},
// 	) => {
// 		const environment = get(actorEnvironmentAtom);
// 		const exportHandler = get(exportLogsHandlerAtom);

// 		if (!environment || !exportHandler) {
// 			throw new Error("Environment or export handler not available");
// 		}

// 		// Build query JSON for the API
// 		// Based on the GET logs endpoint usage, we need to build a query
// 		const query: any = {
// 			actorIds: [actorId],
// 		};

// 		// Add stream filter based on typeFilter
// 		if (typeFilter === "output") {
// 			query.stream = 0; // stdout
// 		} else if (typeFilter === "errors") {
// 			query.stream = 1; // stderr
// 		}

// 		// Add text search if filter is provided
// 		if (filter) {
// 			query.searchText = filter;
// 		}

// 		const result = await exportHandler({
// 			projectNameId: environment.projectNameId,
// 			environmentNameId: environment.environmentNameId,
// 			queryJson: JSON.stringify(query),
// 		});

// 		// Open the presigned URL in a new tab to download
// 		window.open(result.url, "_blank");
// 	},
// );

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

export function ActorDownloadLogsButton({
	actorId,
	typeFilter,
	filter,
	onExportLogs,
	isExporting = false,
}: ActorDownloadLogsButtonProps) {
	// const [isDownloading, setIsDownloading] = useState(false);
	// const [, downloadLogs] = useAtom(downloadLogsAtom);
	// const actorData = useAtomValue(actor);

	// const handleDownload = async () => {
	// 	try {
	// 		setIsDownloading(true);
	// 		await downloadLogs({
	// 			actorId: actorData.id,
	// 			typeFilter,
	// 			filter,
	// 		});
	// 	} catch (error) {
	// 		console.error("Failed to download logs:", error);
	// 	} finally {
	// 		setIsDownloading(false);
	// 	}
	// };
	// const [settings] = useActorDetailsSettings();

	// const [, downloadLogs] = useAtom(downloadLogsAtom);

	return (
		<WithTooltip
			content="Export logs"
			trigger={
				<Button
					className="ml-2 place-self-center"
					variant="outline"
					aria-label="Export logs"
					size="icon-sm"
					// onClick={handleDownload}
					// disabled={isDownloading}
				>
					<Icon
						icon={faSave}
						// className={isDownloading ? "animate-pulse" : ""}
					/>
				</Button>
			}
		/>
	);
}
