import { rivetEeClient } from "@/queries/global";
import { queryOptions } from "@tanstack/react-query";

export const groupBillingUsageQueryOptions = ({
	groupId,
	startTs,
	endTs,
}: {
	groupId: string;
	startTs: Date;
	endTs: Date;
}) =>
	queryOptions({
		queryKey: [
			"group",
			groupId,
			"billing",
			"usage",
			{ startTs, endTs },
		] as const,
		queryFn: ({ queryKey: [_, groupId], signal }) =>
			rivetEeClient.ee.cloud.groups.billing.getUsage(
				groupId,
				{
					queryStartTs: startTs,
					queryEndTs: endTs,
				},
				{ abortSignal: signal },
			),
	});

export const projectBillingUsageQueryOptions = ({
	projectId,
	groupId,
	startTs,
	endTs,
}: {
	projectId: string;
	groupId: string;
	startTs: Date;
	endTs: Date;
}) =>
	queryOptions({
		...groupBillingUsageQueryOptions({ groupId, startTs, endTs }),
		select: (data) => data.games.find((game) => game.gameId === projectId),
	});

export const projectBillingQueryOptions = (
	projectId: string,
	opts: { enabled?: boolean } = {},
) => {
	return queryOptions({
		queryKey: ["project", projectId, "billing"],
		queryFn: ({
			queryKey: [
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				_,
				projectId,
			],
			signal,
		}) =>
			rivetEeClient.ee.cloud.games.billing.get(
				projectId,
				{},
				{
					abortSignal: signal,
				},
			),
		enabled: opts.enabled,
		// HACK: Work around race condition with Stripe API
		retry: 15,
		retryDelay: 1000,
	});
};
