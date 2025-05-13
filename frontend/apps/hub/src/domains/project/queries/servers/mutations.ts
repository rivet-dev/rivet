import { queryClient, rivetClient } from "@/queries/global";
import type { Rivet } from "@rivet-gg/api-full";
import { useMutation } from "@tanstack/react-query";
import {
	projectBuildQueryOptions,
	projectBuildsQueryOptions,
	projectServersQueryOptions,
	serverQueryOptions,
} from "./query-options";

export function useDestroyServerMutation() {
	return useMutation({
		mutationFn: (opts: {
			projectId: string;
			environmentId: string;
			serverId: string;
		}) =>
			rivetClient.servers.destroy(
				opts.projectId,
				opts.environmentId,
				opts.serverId,
			),
		onSuccess: async (_, { projectId, environmentId, serverId }) => {
			await queryClient.invalidateQueries(
				serverQueryOptions({ projectId, environmentId, serverId }),
			);
			await queryClient.invalidateQueries({
				...projectServersQueryOptions({ projectId, environmentId }),
				refetchType: "all",
			});
		},
	});
}

export function usePatchBuildTagsMutation() {
	return useMutation({
		mutationFn: ({
			projectId,
			environmentId,
			buildId,
			...request
		}: {
			projectId: string;
			environmentId: string;
			buildId: string;
		} & Rivet.servers.PatchBuildTagsRequest) =>
			rivetClient.servers.builds.patchTags(
				projectId,
				environmentId,
				buildId,
				request,
			),
	});
}

export function useCreateDynamicServerMutation({
	onSuccess,
}: { onSuccess?: () => void } = {}) {
	return useMutation({
		mutationFn: ({
			projectId,
			environmentId,
			...request
		}: {
			projectId: string;
			environmentId: string;
		} & Rivet.servers.CreateServerRequest) =>
			rivetClient.servers.create(projectId, environmentId, request),
		onSuccess: async (_, { projectId, environmentId }) => {
			await queryClient.invalidateQueries(
				projectServersQueryOptions({ projectId, environmentId }),
			);
			onSuccess?.();
		},
	});
}

export function useUpdateBuildTagsMutation({
	onSuccess,
}: { onSuccess?: () => void } = {}) {
	return useMutation({
		mutationFn: ({
			projectId,
			environmentId,
			buildId,
			...request
		}: {
			projectId: string;
			environmentId: string;
			buildId: string;
		} & Rivet.servers.PatchBuildTagsRequest) =>
			rivetClient.servers.builds.patchTags(
				projectId,
				environmentId,
				buildId,
				request,
			),
		onSuccess: async (_, { projectId, environmentId, buildId }) => {
			await Promise.allSettled([
				queryClient.invalidateQueries(
					projectBuildsQueryOptions({ projectId, environmentId }),
				),
				queryClient.invalidateQueries(
					projectBuildQueryOptions({
						buildId,
						projectId,
						environmentId,
					}),
				),
			]);
			onSuccess?.();
		},
	});
}
