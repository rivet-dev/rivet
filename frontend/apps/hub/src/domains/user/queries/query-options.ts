import { timing } from "@rivet-gg/components";
import { queryOptions } from "@tanstack/react-query";
import { rivetClient } from "../../../queries/global";
import { getMetaWatchIndex } from "../../../queries/utils";
import { Changelog } from "./type";

export const selfProfileQueryOptions = (opts: { enabled?: boolean } = {}) => {
	return queryOptions({
		...opts,
		queryKey: ["selfProfile"],
		queryFn: ({ meta, signal }) => {
			return rivetClient.identity.getSelfProfile(
				{
					watchIndex: getMetaWatchIndex(meta),
				},
				{ abortSignal: signal },
			);
		},
		meta: { watch: true },
	});
};

export const selfProfileIdentityIdQueryOptions = () => {
	return queryOptions({
		...selfProfileQueryOptions(),
		select: (data) => data.identity.identityId,
	});
};

export const changelogQueryOptions = () => {
	return queryOptions({
		queryKey: ["changelog", __APP_GIT_COMMIT__],
		staleTime: timing.hours(1),
		queryFn: async () => {
			const response = await fetch("https://rivet.gg/changelog.json");
			if (!response.ok) {
				throw new Error("Failed to fetch changelog");
			}
			const result = Changelog.safeParse(await response.json());
			return result.success ? result.data : [];
		},
	});
};
