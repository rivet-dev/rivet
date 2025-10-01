import { memo } from "react";
import type { ActorId } from "./queries";

export type LogsTypeFilter = "all" | "output" | "errors";

interface ActorLogsProps {
	actorId: ActorId;
	typeFilter?: LogsTypeFilter;
	filter?: string;
}

export const ActorLogs = memo((_props: ActorLogsProps) => {
	return null;
});
