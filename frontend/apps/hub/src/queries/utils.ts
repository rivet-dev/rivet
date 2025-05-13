import type { Rivet } from "@rivet-gg/api-full";
import type { QueryMeta } from "@tanstack/react-query";

export const getMetaWatchIndex = (
	meta: QueryMeta | undefined,
): Rivet.WatchQuery => {
	return meta?.__watcher?.index;
};
