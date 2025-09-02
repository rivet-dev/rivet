import { startTransition, useState } from "react";
import { LogsView, ToggleGroup, ToggleGroupItem } from "@/components";
import type { ActorAtom } from "./actor-context";
import { ActorDetailsSettingsButton } from "./actor-details-settings-button";
import { ActorDownloadLogsButton } from "./actor-download-logs-button";
import { ActorLogs, type LogsTypeFilter } from "./actor-logs";
import type { ActorId } from "./queries";

interface ActorLogsTabProps {
	actorId: ActorId;
}

export function ActorLogsTab({ actorId }: ActorLogsTabProps) {
	const [search, setSearch] = useState("");
	const [logsFilter, setLogsFilter] = useState<LogsTypeFilter>("all");

	return (
		<div className="flex flex-col h-full">
			<div className="border-b">
				<div className="flex items-stretch px-2">
					<div className="border-r flex flex-1">
						<input
							type="text"
							className="bg-transparent outline-none px-2 text-xs placeholder:text-muted-foreground font-sans flex-1"
							placeholder="Filter output"
							spellCheck={false}
							onChange={(e) =>
								startTransition(() => setSearch(e.target.value))
							}
						/>
					</div>
					<ToggleGroup
						type="single"
						value={logsFilter}
						size="xs"
						onValueChange={(value) => {
							if (!value) {
								setLogsFilter("all");
							} else {
								setLogsFilter(value as LogsTypeFilter);
							}
						}}
						className="gap-0 text-xs p-2 border-r"
					>
						<ToggleGroupItem
							value="all"
							className="text-xs border border-r-0 rounded-se-none rounded-ee-none"
						>
							all
						</ToggleGroupItem>
						<ToggleGroupItem
							value="output"
							className="text-xs border rounded-none"
						>
							output
						</ToggleGroupItem>
						<ToggleGroupItem
							value="errors"
							className=" text-xs border rounded-es-none rounded-ss-none border-l-0"
						>
							errors
						</ToggleGroupItem>
					</ToggleGroup>
					<ActorDownloadLogsButton
						actorId={actorId}
						typeFilter={logsFilter}
						filter={search}
						// onExportLogs={onExportLogs}
						// isExporting={isExporting}
					/>
					<ActorDetailsSettingsButton className="ml-2" />
				</div>
			</div>
			<div className="flex-1 min-h-0 overflow-hidden flex relative">
				<ActorLogs
					actorId={actorId}
					typeFilter={logsFilter}
					filter={search}
				/>
			</div>
		</div>
	);
}

ActorLogsTab.Skeleton = () => {
	return (
		<div className="px-4 pt-4">
			<LogsView.Skeleton />
		</div>
	);
};
