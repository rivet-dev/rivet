import { useAuth } from "@/domains/auth/contexts/auth";
import { queryClient, rivetEeClient } from "@/queries/global";
import { OuterbaseError } from "@/queries/types";
import type { Rivet as RivetEe } from "@rivet-gg/api-ee";
import { getConfig, toast } from "@rivet-gg/components";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { usePostHog } from "posthog-js/react";
import { extractPostgressCredentials } from "../../helpers/extract-postgress-credentials";
import {
	projectBackendEnvVariablesQueryOptions,
	projectBackendProjectEnvDatabasePreviewQueryOptions,
	projectBackendProjectEnvDatabaseQueryOptions,
	projectBackendQueryOptions,
} from "./query-options";
import { OuterbaseStarlinkResponse } from "./types";

export const useCreateBackendMutation = ({
	onSuccess,
}: {
	onSuccess?: (data: RivetEe.ee.backend.CreateResponse) => void;
}) =>
	useMutation({
		mutationFn: ({
			projectId,
			environmentId,
			...data
		}: RivetEe.ee.backend.CreateRequest & {
			projectId: string;
			environmentId: string;
		}) => rivetEeClient.ee.backend.create(projectId, environmentId, data),
		onSuccess: async (data) => {
			onSuccess?.(data);
		},
	});

export const useBackendUpdateVariablesMutation = () =>
	useMutation({
		mutationFn: ({
			projectId,
			environmentId,
			...data
		}: RivetEe.ee.backend.UpdateVariablesRequest & {
			projectId: string;
			environmentId: string;
		}) =>
			rivetEeClient.ee.backend.updateVariables(
				projectId,
				environmentId,
				data,
			),
		onSuccess: async (data, { projectId, environmentId }) => {
			await Promise.all([
				queryClient.invalidateQueries(
					projectBackendQueryOptions({ projectId, environmentId }),
				),
				queryClient.invalidateQueries(
					projectBackendEnvVariablesQueryOptions({
						projectId,
						environmentId,
					}),
				),
			]);
		},
	});

export const useProjectBackendEnvDatabasePreviewMutation = (
	opts: { onSuccess?: (url: string) => void } = {},
) => {
	const postHog = usePostHog();
	const queryClient = useQueryClient();
	const { profile } = useAuth();
	return useMutation({
		mutationKey: ["backend-project", "env", "database-preview"],
		mutationFn: async ({
			projectId,
			environmentId,
		}: {
			projectId: string;
			environmentId: string;
		}) => {
			const response = await queryClient.fetchQuery(
				projectBackendProjectEnvDatabaseQueryOptions({
					projectId,
					environmentId,
				}),
			);

			if (!response.url) {
				throw new Error("Database URL not found");
			}

			const credentials = extractPostgressCredentials(response.url);

			const starlinkResponse = await fetch(
				"https://app.outerbase.com/api/v1/starlink",
				{
					method: "POST",
					headers: {
						"Content-Type": "application/json",
						"x-provider-token": getConfig().outerbaseProviderToken,
					},
					body: JSON.stringify({
						credentials: {
							...credentials,
							ssl_config: {
								require: true,
							},
						},
						providerUniqueId: profile?.identity.identityId,
					}),
				},
			);

			if (!starlinkResponse.ok) {
				throw await starlinkResponse.json();
			}

			const parsedResponse = OuterbaseStarlinkResponse.parse(
				await starlinkResponse.json(),
			);
			return parsedResponse.response.url;
		},
		onSuccess: async (data, variables) => {
			await queryClient.setQueryData(
				projectBackendProjectEnvDatabasePreviewQueryOptions(variables)
					.queryKey,
				data,
			);
			opts.onSuccess?.(data);
		},
		onError: (error, variables) => {
			const result = OuterbaseError.safeParse(error);
			if (
				result.success &&
				result.data.error.description === "RATE_LIMIT_EXCEEDED"
			) {
				postHog.capture("outerbase_rate_limit_exceeded", variables);
				return toast.error(
					"Rate limit exceeded. Please try again later.",
				);
			}
		},
	});
};
