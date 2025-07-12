import { Button, WithTooltip } from "@rivet-gg/components";
import { Icon, faSave } from "@rivet-gg/icons";
import { type LogsTypeFilter } from "./actor-logs";
import type { ActorAtom } from "./actor-context";
import { actorEnvironmentAtom, exportLogsHandlerAtom } from "./actor-context";
import { atom, useAtom, useAtomValue } from "jotai";
import { useState } from "react";

const downloadLogsAtom = atom(
	null,
	async (
		get,
		_set,
		{
			actorId,
			typeFilter,
			filter,
		}: {
			actorId: string;
			typeFilter?: LogsTypeFilter;
			filter?: string;
		},
	) => {
		const environment = get(actorEnvironmentAtom);
		const exportHandler = get(exportLogsHandlerAtom);

		if (!environment || !exportHandler) {
			throw new Error("Environment or export handler not available");
		}

		// Build query JSON for the API
		// Based on the GET logs endpoint usage, we need to build a query
		const query: any = {
			actorIds: [actorId],
		};

		// Add stream filter based on typeFilter
		if (typeFilter === "output") {
			query.stream = 0; // stdout
		} else if (typeFilter === "errors") {
			query.stream = 1; // stderr
		}

		// Add text search if filter is provided
		if (filter) {
			query.searchText = filter;
		}

		const result = await exportHandler({
			projectNameId: environment.projectNameId,
			environmentNameId: environment.environmentNameId,
			queryJson: JSON.stringify(query),
		});

		// Open the presigned URL in a new tab to download
		window.open(result.url, "_blank");
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
	const [isDownloading, setIsDownloading] = useState(false);
	const [, downloadLogs] = useAtom(downloadLogsAtom);
	const actorData = useAtomValue(actor);

	const handleDownload = async () => {
		try {
			setIsDownloading(true);
			await downloadLogs({
				actorId: actorData.id,
				typeFilter,
				filter,
			});
		} catch (error) {
			console.error("Failed to download logs:", error);
		} finally {
			setIsDownloading(false);
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
					disabled={isDownloading}
				>
					<Icon
						icon={faSave}
						className={isDownloading ? "animate-pulse" : ""}
					/>
				</Button>
			}
		/>
	);
}
