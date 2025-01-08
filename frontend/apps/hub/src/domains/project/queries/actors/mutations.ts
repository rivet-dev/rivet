import { queryClient, rivetClient } from "@/queries/global";
import type { Rivet } from "@rivet-gg/api";
import { toast } from "@rivet-gg/components";
import { useMutation } from "@tanstack/react-query";
import {
	actorBuildsQueryOptions,
	actorQueryOptions,
	projectActorsQueryOptions,
} from "./query-options";

export function useDestroyActorMutation() {
	return useMutation({
		mutationFn: (opts: {
			projectNameId: string;
			environmentNameId: string;
			actorId: string;
		}) =>
			rivetClient.actor.destroy(opts.actorId, {
				environment: opts.environmentNameId,
				project: opts.projectNameId,
			}),
		onSuccess: async (_, { projectNameId, environmentNameId, actorId }) => {
			await queryClient.invalidateQueries(
				actorQueryOptions({
					projectNameId,
					environmentNameId,
					actorId,
				}),
			);
			await queryClient.invalidateQueries({
				...projectActorsQueryOptions({
					projectNameId,
					environmentNameId,
				}),
				refetchType: "all",
			});
		},
	});
}

export function usePatchActorBuildTagsMutation({
	onSuccess,
}: { onSuccess?: () => void } = {}) {
	return useMutation({
		mutationFn: ({
			projectNameId,
			environmentNameId,
			buildId,
			...request
		}: {
			projectNameId: string;
			environmentNameId: string;
			buildId: string;
		} & Rivet.servers.PatchBuildTagsRequest) =>
			rivetClient.actor.builds.patchTags(buildId, {
				project: projectNameId,
				environment: environmentNameId,
				body: request,
			}),
		onSuccess: async (_, { projectNameId, environmentNameId, buildId }) => {
			await Promise.all([
				queryClient.invalidateQueries(
					projectActorsQueryOptions({
						projectNameId,
						environmentNameId,
					}),
				),
				// until we migrate old endpoints to use nameIds
				queryClient.invalidateQueries({
					predicate(query) {
						return (
							query.queryKey[0] === "project" &&
							query.queryKey[2] === "environment" &&
							query.queryKey[4] === "builds"
						);
					},
				}),
			]);
			onSuccess?.();
		},
	});
}

export function useUpgradeAllActorsMutation({
	onSuccess,
}: { onSuccess?: () => void } = {}) {
	return useMutation({
		mutationFn: ({
			projectNameId,
			environmentNameId,
			...request
		}: {
			projectNameId: string;
			environmentNameId: string;
		} & Rivet.actor.UpgradeAllActorsRequest) =>
			rivetClient.actor.upgradeAll({
				project: projectNameId,
				environment: environmentNameId,
				body: request,
			}),
		onSuccess: async (response, { projectNameId, environmentNameId }) => {
			await Promise.allSettled([
				queryClient.invalidateQueries(
					projectActorsQueryOptions({
						projectNameId,
						environmentNameId,
					}),
				),
				queryClient.invalidateQueries(
					actorBuildsQueryOptions({
						projectNameId,
						environmentNameId,
					}),
				),
			]);

			toast.success(
				response.count
					? `Build successfully tagged. Upgraded ${response.count} actors to the latest build.`
					: "Build successfully tagged. No actors to upgrade.",
			);
			onSuccess?.();
		},
	});
}
