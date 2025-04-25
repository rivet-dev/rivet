import { mutationOptions, queryClient, rivetClient } from "@/queries/global";
import type { Rivet } from "@rivet-gg/api";
import { toast } from "@rivet-gg/components";
import { useMutation } from "@tanstack/react-query";
import {
	actorBuildsQueryOptions,
	actorQueryOptions,
	projectActorsQueryOptions,
	routesQueryOptions,
} from "./query-options";
import { customAlphabet } from "nanoid";

const nanoid = customAlphabet("0123456789abcdefghijklmnoprstwuxyz", 5);

export function destroyActorMutationOptions() {
	return mutationOptions({
		mutationFn: (opts: {
			projectNameId: string;
			environmentNameId: string;
			actorId: string;
		}) =>
			rivetClient.actors.destroy(opts.actorId, {
				environment: opts.environmentNameId,
				project: opts.projectNameId,
			}),
		onSuccess: async (_, { projectNameId, environmentNameId, actorId }) => {
			const { queryKey: projectActorsQueryKey } =
				projectActorsQueryOptions({
					projectNameId: "<placeholder>",
					environmentNameId: "<placeholder>",
				});

			await queryClient.invalidateQueries(
				actorQueryOptions({
					projectNameId,
					environmentNameId,
					actorId,
				}),
			);
			await queryClient.invalidateQueries({
				predicate: (query) => {
					return query.queryKey.every((key, i) => {
						return (
							projectActorsQueryKey[i] === key ||
							projectActorsQueryKey[i] === "<placeholder>"
						);
					});
				},
				refetchType: "all",
			});
		},
	});
}

export function usePatchActorBuildTagsMutation({
	onSuccess,
}: { onSuccess?: () => void } = {}) {
	return useMutation({
		mutationFn: async ({
			projectNameId,
			environmentNameId,
			buildId,
			...request
		}: {
			projectNameId: string;
			environmentNameId: string;
			buildId: string;
		} & Rivet.servers.PatchBuildTagsRequest) => {
			// TODO: Cache this
			// Get original build
			const ogBuild = await rivetClient.builds.get(buildId, {
				project: projectNameId,
				environment: environmentNameId,
			});

			// If setting build to current, remove current tag from all other builds with the same name
			if (
				ogBuild.build.tags.name &&
				(request.tags as Record<string, string> | undefined)
					?.current === "true"
			) {
				const currentBuilds = await rivetClient.builds.list({
					project: projectNameId,
					environment: environmentNameId,
					tagsJson: JSON.stringify({
						name: ogBuild.build.tags.name,
						current: "true",
					}),
				});

				for (const build of currentBuilds.builds) {
					await rivetClient.builds.patchTags(build.id, {
						project: projectNameId,
						environment: environmentNameId,
						body: {
							tags: {
								current: null,
							},
						},
					});
				}
			}

			// Update tags
			return await rivetClient.builds.patchTags(buildId, {
				project: projectNameId,
				environment: environmentNameId,
				body: request,
			});
		},
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
		} & Rivet.actors.UpgradeAllActorsRequest) =>
			rivetClient.actors.upgradeAll({
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

export function useCreateActorFromSdkMutation({
	onSuccess,
}: { onSuccess?: () => void }) {
	return useMutation({
		mutationFn: async ({
			projectNameId,
			environmentNameId,
			buildId,
			region,
			parameters,
		}: {
			projectNameId: string;
			environmentNameId: string;
			buildId: string;
			region: string;
			parameters: unknown;
		}) => {
			// const managerUrl = await queryClient.fetchQuery(
			// 	actorManagerUrlQueryOptions({
			// 		projectNameId,
			// 		environmentNameId,
			// 	}),
			// );
			// const { build } = await queryClient.fetchQuery(
			// 	actorBuildQueryOptions({
			// 		projectNameId,
			// 		environmentNameId,
			// 		buildId,
			// 	}),
			// );
			// const cl = new Client(managerUrl);
			// await cl.create({
			// 	parameters,
			// 	create: { tags: { name: build.tags.name || build.id }, region },
			// });
		},
		onSuccess: async () => {
			await queryClient.invalidateQueries({
				predicate(query) {
					return (
						query.queryKey[0] === "project" &&
						query.queryKey[2] === "environment" &&
						query.queryKey[4] === "actors"
					);
				},
			});
			onSuccess?.();
		},
	});
}

export const usePatchRouteMutation = ({
	onSuccess,
}: { onSuccess?: () => void } = {}) => {
	return useMutation({
		mutationFn: async ({
			projectNameId,
			environmentNameId,
			id,
			...request
		}: {
			projectNameId: string;
			environmentNameId: string;
			id: string;
		} & Rivet.routes.UpdateRouteBody) =>
			rivetClient.routes.update(id, {
				body: request,
				project: projectNameId,
				environment: environmentNameId,
			}),
		onSuccess: async (_, { projectNameId, environmentNameId }) => {
			await queryClient.invalidateQueries(
				routesQueryOptions({
					projectNameId,
					environmentNameId,
				}),
			);
			onSuccess?.();
		},
	});
};

export const useCreateRouteMutation = ({
	onSuccess,
}: { onSuccess?: () => void } = {}) => {
	return useMutation({
		mutationFn: async ({
			projectNameId,
			environmentNameId,
			...request
		}: {
			projectNameId: string;
			environmentNameId: string;
		} & Rivet.routes.UpdateRouteBody) =>
			rivetClient.routes.update(`route-${nanoid()}`, {
				body: request,
				project: projectNameId,
				environment: environmentNameId,
			}),
		onSuccess: async (_, { projectNameId, environmentNameId }) => {
			await queryClient.invalidateQueries(
				routesQueryOptions({
					projectNameId,
					environmentNameId,
				}),
			);
			onSuccess?.();
		},
	});
};

export const useDeleteRouteMutation = ({
	onSuccess,
}: { onSuccess?: () => void } = {}) => {
	return useMutation({
		mutationFn: async ({
			projectNameId,
			environmentNameId,
			routeId,
		}: {
			projectNameId: string;
			environmentNameId: string;
			routeId: string;
		}) =>
			rivetClient.routes.delete(routeId, {
				project: projectNameId,
				environment: environmentNameId,
			}),
		onSuccess: async (_, { projectNameId, environmentNameId }) => {
			await queryClient.invalidateQueries(
				routesQueryOptions({
					projectNameId,
					environmentNameId,
				}),
			);
			onSuccess?.();
		},
	});
};
