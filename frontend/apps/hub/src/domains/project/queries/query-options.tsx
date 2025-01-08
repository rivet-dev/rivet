import { isRivetError } from "@/lib/utils";
import { rivetClient, rivetEeClient } from "@/queries/global";
import { getMetaWatchIndex } from "@/queries/utils";
import { RivetError } from "@rivet-gg/api";
import { loadModuleCategories } from "@rivet-gg/components";
import { queryOptions } from "@tanstack/react-query";

export const projectsQueryOptions = () => {
	return queryOptions({
		queryKey: ["projects"],
		queryFn: ({ meta, signal }) =>
			rivetClient.cloud.games.getGames(
				{
					watchIndex: getMetaWatchIndex(meta),
				},
				{ abortSignal: signal },
			),
		select: (data) => data.games,
	});
};

export const projectsByGroupQueryOptions = () => {
	return queryOptions({
		...projectsQueryOptions(),
		select: (data) => {
			return data.groups.map((group) => {
				return {
					...group,
					projects: data.games.filter(
						(game) => game.developer.groupId === group.groupId,
					),
				};
			});
		},
	});
};

export const groupsCountQueryOptions = () => {
	return queryOptions({
		...projectsByGroupQueryOptions(),
		select: (data) => data.groups.length,
	});
};

export const groupProjectsQueryOptions = (groupId: string) => {
	return queryOptions({
		queryKey: ["team", groupId, "projects"],
		retry(failureCount, error) {
			if (isRivetError(error) && error.statusCode === 404) {
				return false;
			}
			return failureCount < 3;
		},
		queryFn: async ({ signal, meta }) => {
			const data = await rivetClient.cloud.games.getGames(
				{
					watchIndex: getMetaWatchIndex(meta),
				},
				{ abortSignal: signal },
			);

			const group = data.groups.find(
				(group) => group.groupId === groupId,
			);
			if (!group) {
				throw new RivetError({
					statusCode: 404,
					body: {
						message: "Group not found",
					},
				});
			}

			const projects = data.games.filter(
				(game) => game.developer.groupId === group.groupId,
			);
			return {
				...group,
				projects,
			};
		},
	});
};

export const groupOnwerQueryOptions = (groupId: string) => {
	return queryOptions({
		...groupProjectsQueryOptions(groupId),
		select: (data) => data.ownerIdentityId,
	});
};

export const projectsCountQueryOptions = (groupId: string) => {
	return queryOptions({
		...groupProjectsQueryOptions(groupId),
		select: (data) => data.projects.length,
	});
};

export const projectByIdQueryOptions = (projectNameId: string) => {
	return queryOptions({
		...projectsByGroupQueryOptions(),
		select: (data) =>
			// biome-ignore lint/style/noNonNullAssertion: when we get here, we know the project exists
			data.games.find((game) => game.nameId === projectNameId)!,
	});
};

export const projectQueryOptions = (projectId: string) => {
	return queryOptions({
		queryKey: ["project", projectId],
		queryFn: ({ queryKey: [_, projectId], signal, meta }) =>
			rivetClient.cloud.games.getGameById(
				projectId,
				{
					watchIndex: getMetaWatchIndex(meta),
				},
				{ abortSignal: signal },
			),
		select: (data) => ({
			...data.game,
			namespaces: data.game.namespaces.map((environment) => ({
				...environment,
				version: data.game.versions.find(
					(version) => version.versionId === environment.versionId,
				),
			})),
		}),
	});
};

export const environmentByIdQueryOptions = ({
	projectId,
	environmentNameId,
}: {
	projectId: string;
	environmentNameId: string;
}) => {
	return queryOptions({
		...projectQueryOptions(projectId),
		select: (data) =>
			// biome-ignore lint/style/noNonNullAssertion: when we get here, we know the environment exists
			projectQueryOptions(projectId)
				.select?.(data)
				.namespaces.find(
					(namespace) => namespace.nameId === environmentNameId,
				)!,
	});
};

export const projectVersionsQueryOptions = (projectId: string) => {
	return queryOptions({
		...projectQueryOptions(projectId),
		select: (data) =>
			projectQueryOptions(projectId)
				.select?.(data)
				.versions.sort(
					(a, b) => b.createTs.getTime() - a.createTs.getTime(),
				),
	});
};

export const projectRegionsQueryOptions = (projectId: string) => {
	return queryOptions({
		...projectQueryOptions(projectId),
		select: (data) =>
			// biome-ignore lint/style/noNonNullAssertion: when we get here, we know the regions exist
			projectQueryOptions(projectId).select?.(data).availableRegions!,
	});
};

export const projectRegionQueryOptions = ({
	projectId,
	regionId,
}: {
	projectId: string;
	regionId: string;
}) => {
	return queryOptions({
		...projectRegionsQueryOptions(projectId),
		select: (data) =>
			projectRegionsQueryOptions(projectId)
				.select?.(data)
				.find((region) => region.regionId === regionId),
	});
};

export const projectVersionQueryOptions = ({
	projectId,
	versionId,
}: {
	projectId: string;
	versionId: string;
}) =>
	queryOptions({
		...projectQueryOptions(projectId),
		select: (data) =>
			// biome-ignore lint/style/noNonNullAssertion: when we get here, we know the version exists
			projectQueryOptions(projectId)
				.select?.(data)
				.versions.find((version) => version.versionId === versionId)!,
	});

export const projectTokenCloudQueryOptions = ({
	projectId,
}: { projectId: string }) => {
	return queryOptions({
		staleTime: 0,
		gcTime: 0,
		queryKey: ["project", projectId, "token", "cloud"],
		queryFn: ({
			queryKey: [
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				_,
				projectId,
			],
		}) => rivetClient.cloud.games.tokens.createCloudToken(projectId),
		select: (data) => data.token,
	});
};

export const projectEnvTokenServiceQueryOptions = ({
	projectId,
	environmentId,
}: {
	projectId: string;
	environmentId: string;
}) => {
	return queryOptions({
		staleTime: 0,
		gcTime: 0,
		queryKey: [
			"project",
			projectId,
			"environment",
			environmentId,
			"token",
			"service",
		],
		queryFn: ({
			queryKey: [
				// eslint-disable-next-line @typescript-eslint/no-unused-vars
				_,
				projectId,
				__,
				environmentId,
			],
		}) =>
			rivetClient.games.environments.tokens.createServiceToken(
				projectId,
				environmentId,
			),
		select: (data) => data.token,
	});
};

export const projectMetadataQueryOptions = ({
	projectId,
	environmentId,
}: {
	projectId: string;
	environmentId: string;
}) => {
	return queryOptions({
		// no need to refetch this often
		// the metadata is not expected to change often
		staleTime: 24 * 60 * 60 * 1000,
		queryKey: [
			"project",
			projectId,
			"environment",
			environmentId,
			"metadata",
		],
		queryFn: async ({ queryKey: [_, projectId, __, environmentId] }) => {
			const { game } =
				await rivetClient.cloud.games.getGameById(projectId);
			const legacyLobbiesEnabled = game.versions.length > 1;

			const bootstrap = await rivetClient.cloud.bootstrap();
			try {
				if (bootstrap.cluster === "enterprise") {
					// should throw when there is no backend
					await rivetEeClient.ee.backend.get(
						projectId,
						environmentId,
					);

					// sometimes even if there is a backend, there are no variables
					// that means the user has not used the backend yet
					// so its safe to disable the backend modules
					const { variables } =
						await rivetEeClient.ee.backend.getVariables(
							projectId,
							environmentId,
						);
					const backendModulesEnabled =
						Object.keys(variables).length > 0;

					return {
						legacyLobbiesEnabled,
						backendModulesEnabled,
					};
				}
			} catch {}

			return {
				legacyLobbiesEnabled,
				backendModulesEnabled: false,
			};
		},
	});
};

export const modulesCategoriesQueryOptions = () => {
	return queryOptions({
		queryKey: ["modules", "categories"],
		queryFn: () => loadModuleCategories(),
	});
};

const FEATURED_MODULES = ["lobbies", "friends", "analytics"];

export const featuredModulesQueryOptions = () => {
	return queryOptions({
		...modulesCategoriesQueryOptions(),
		queryKey: ["modules", "featured"],
		select: (data) => {
			return data
				.flatMap((category) => category.modules)
				.filter((module) => FEATURED_MODULES.includes(module.id));
		},
	});
};
