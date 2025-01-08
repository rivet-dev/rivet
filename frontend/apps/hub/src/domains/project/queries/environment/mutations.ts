import { queryClient, rivetClient } from "@/queries/global";
import type { Rivet } from "@rivet-gg/api";
import { toast } from "@rivet-gg/components";
import { useMutation } from "@tanstack/react-query";
import { projectQueryOptions } from "../query-options";
import {
	projectEnvironmentLobbyQueryOptions,
	projectEnvironmentQueryOptions,
} from "./query-options";

export const useEnvironmentMatchmakerUpdateConfigMutation = () => {
	return useMutation({
		mutationFn: ({
			projectId,
			environmentId,
			lobbyCountMax,
			maxPlayers,
		}: {
			projectId: string;
			environmentId: string;
		} & Rivet.cloud.games.namespaces.UpdateGameNamespaceMatchmakerConfigRequest) =>
			rivetClient.cloud.games.namespaces.updateGameNamespaceMatchmakerConfig(
				projectId,
				environmentId,
				{ lobbyCountMax, maxPlayers },
			),
		onSuccess: async (data, values) => {
			await queryClient.invalidateQueries(
				projectQueryOptions(values.projectId),
			);
			await queryClient.invalidateQueries(
				projectEnvironmentQueryOptions({
					projectId: values.projectId,
					environmentId: values.environmentId,
				}),
			);
		},
	});
};

export const useEnvironmentMatchmakeDeleteLobbyMutation = () => {
	return useMutation({
		mutationFn: ({
			projectId,
			lobbyId,
		}: {
			projectId: string;
			environmentId: string;
			lobbyId: string;
		}) =>
			rivetClient.cloud.games.matchmaker.deleteMatchmakerLobby(
				projectId,
				lobbyId,
			),
		onSuccess: async (_, values) => {
			await queryClient.invalidateQueries(
				projectEnvironmentLobbyQueryOptions({
					projectId: values.projectId,
					environmentId: values.environmentId,
					lobbyId: values.lobbyId,
				}),
			);
		},
	});
};

export const useEnvironmentRemoveDomainMutation = () => {
	return useMutation({
		mutationFn: ({
			projectId,
			environmentId,
			domain,
		}: {
			projectId: string;
			environmentId: string;
			domain: string;
		}) =>
			rivetClient.cloud.games.namespaces.removeNamespaceDomain(
				projectId,
				environmentId,
				domain,
			),
		onSuccess: async (data, values) => {
			await queryClient.invalidateQueries(
				projectQueryOptions(values.projectId),
			);
			await queryClient.invalidateQueries(
				projectEnvironmentQueryOptions({
					projectId: values.projectId,
					environmentId: values.environmentId,
				}),
			);
		},
	});
};

export const useUpdateProjectEnvironmentVersionMutation = ({
	onSuccess,
}: {
	onSuccess?: () => void;
}) => {
	return useMutation({
		mutationFn: ({
			projectId,
			environmentId,
			versionId,
		}: {
			projectId: string;
			environmentId: string;
		} & Rivet.cloud.games.namespaces.UpdateGameNamespaceVersionRequest) =>
			rivetClient.cloud.games.namespaces.updateGameNamespaceVersion(
				projectId,
				environmentId,
				{ versionId },
			),
		onSuccess: async (data, values) => {
			await queryClient.invalidateQueries(
				projectQueryOptions(values.projectId),
			);
			onSuccess?.();
		},
	});
};

export const useEnvironmentDomainPublicAuthMutation = ({
	onSuccess,
}: {
	onSuccess?: () => void;
} = {}) => {
	return useMutation({
		mutationFn: ({
			projectId,
			environmentId,
			enabled,
		}: {
			projectId: string;
			environmentId: string;
			enabled: boolean;
		}) =>
			rivetClient.cloud.games.namespaces.toggleNamespaceDomainPublicAuth(
				projectId,
				environmentId,
				{
					enabled,
				},
			),
		onError: () => {
			toast.error("Failed to update domain-based authentication");
		},
		onSuccess: async (data, values) => {
			await queryClient.invalidateQueries(
				projectQueryOptions(values.projectId),
			);
			await queryClient.invalidateQueries(
				projectEnvironmentQueryOptions({
					projectId: values.projectId,
					environmentId: values.environmentId,
				}),
			);
			onSuccess?.();
		},
	});
};

export const useEnvironmentAuthTypeMutation = ({
	onSuccess,
}: {
	onSuccess?: () => void;
} = {}) => {
	return useMutation({
		mutationFn: ({
			projectId,
			environmentId,
			authType,
		}: {
			projectId: string;
			environmentId: string;
			authType: Rivet.cloud.CdnAuthType;
		}) =>
			rivetClient.cloud.games.namespaces.setNamespaceCdnAuthType(
				projectId,
				environmentId,
				{
					authType,
				},
			),
		onError: () => {
			toast.error("Failed to update authentication type");
		},
		onSuccess: async (data, values) => {
			await queryClient.invalidateQueries(
				projectQueryOptions(values.projectId),
			);
			await queryClient.invalidateQueries(
				projectEnvironmentQueryOptions({
					projectId: values.projectId,
					environmentId: values.environmentId,
				}),
			);
			onSuccess?.();
		},
	});
};

export const useEnvironmentUpdateCdnAuthUserMutation = () => {
	return useMutation({
		mutationFn: ({
			projectId,
			environmentId,
			user,
			password,
		}: {
			projectId: string;
			environmentId: string;
		} & Rivet.cloud.games.namespaces.UpdateNamespaceCdnAuthUserRequest) =>
			rivetClient.cloud.games.namespaces.updateNamespaceCdnAuthUser(
				projectId,
				environmentId,
				{
					user,
					password,
				},
			),
	});
};

export const useEnvironmentRemoveCdnAuthUserMutation = () => {
	return useMutation({
		mutationFn: ({
			projectId,
			environmentId,
			user,
		}: {
			projectId: string;
			environmentId: string;
			user: string;
		}) =>
			rivetClient.cloud.games.namespaces.removeNamespaceCdnAuthUser(
				projectId,
				environmentId,
				user,
			),
	});
};

export const useEnvironmentAddDomainMutation = () => {
	return useMutation({
		mutationFn: ({
			projectId,
			environmentId,
			domain,
		}: {
			projectId: string;
			environmentId: string;
		} & Rivet.cloud.games.namespaces.AddNamespaceDomainRequest) =>
			rivetClient.cloud.games.namespaces.addNamespaceDomain(
				projectId,
				environmentId,
				{ domain },
			),
	});
};
