import type { Clerk } from "@clerk/clerk-js";
import { type Rivet, RivetClient } from "@rivet-gg/cloud";
import { fetcher } from "@rivetkit/engine-api-full/core";
import {
	infiniteQueryOptions,
	mutationOptions,
	type QueryKey,
	queryOptions,
	type UseQueryOptions,
} from "@tanstack/react-query";
import { clerk } from "@/lib/auth";
import { cloudEnv } from "@/lib/env";
import { queryClient } from "@/queries/global";
import { RECORDS_PER_PAGE } from "./default-data-provider";
import {
	type CreateNamespace,
	createClient as createEngineClient,
	createNamespaceContext as createEngineNamespaceContext,
	type Namespace,
} from "./engine-data-provider";
import { no404Retry } from "./utilities";

function createClient({ clerk }: { clerk: Clerk }) {
	return new RivetClient({
		baseUrl: () => cloudEnv().VITE_APP_CLOUD_API_URL,
		environment: "",
		token: async () => {
			return (await clerk.session?.getToken()) || "";
		},
		// @ts-expect-error
		fetcher: async (args) => {
			Object.keys(args.headers || {}).forEach((key) => {
				if (key.toLowerCase().startsWith("x-fern-")) {
					delete args.headers?.[key];
				}
			});
			return await fetcher(
				// @ts-expect-error
				args,
			);
		},
	});
}

export const createGlobalContext = ({ clerk }: { clerk: Clerk }) => {
	const client = createClient({ clerk });
	return {
		client,
		organizationQueryOptions(opts: { org: string }) {
			return queryOptions({
				queryKey: ["organization", opts.org],
				queryFn: async () => {
					return clerk.getOrganization(opts.org);
				},
			});
		},
		billingCustomerPortalSessionQueryOptions() {
			return queryOptions({
				staleTime: 5 * 60 * 1000, // 5 minutes
				gcTime: 5 * 60 * 1000, // 5 minutes
				queryKey: ["billing-customer-portal-session"],
				queryFn: async () => {
					const session =
						await client.billing.createCustomerPortalSession();
					if (!session.url) {
						throw new Error(
							"No URL returned for customer portal session",
						);
					}
					return session.url;
				},
			});
		},
		billingDetailsQueryOptions({
			organization,
			project,
		}: {
			organization: string;
			project: string;
		}) {
			return queryOptions({
				queryKey: [{ organization, project }, "billing-details"],
				queryFn: async () => {
					const response = await client.billing.details(project, {
						org: organization,
					});
					return response;
				},
			});
		},
	};
};

export const createOrganizationContext = ({
	client,
	organization,
	...parent
}: {
	organization: string;
} & ReturnType<typeof createGlobalContext>) => {
	const orgProjectNamespacesQueryOptions = (opts: {
		organization: string;
		project: string;
	}) =>
		infiniteQueryOptions({
			queryKey: [opts, "namespaces"],
			initialPageParam: undefined as string | undefined,
			queryFn: async ({ pageParam }) => {
				const data = await client.namespaces.list(opts.project, {
					org: opts.organization,
					limit: 100,
					cursor: pageParam ?? undefined,
				});
				return {
					pagination: data.pagination,
					namespaces: data.namespaces.map((ns) => ({
						id: ns.id,
						name: ns.name,
						displayName: ns.displayName,
						createdAt: ns.createdAt,
					})),
				};
			},
			getNextPageParam: (lastPage) => {
				if (lastPage.namespaces.length < 100) {
					return undefined;
				}
				return lastPage.pagination.cursor;
			},
			select: (data) => data.pages.flatMap((page) => page.namespaces),
		});

	const projectsQueryOptions = (opts: { organization: string }) =>
		infiniteQueryOptions({
			queryKey: [opts, "projects"],
			initialPageParam: undefined as string | undefined,
			queryFn: async ({ pageParam }) => {
				const data = await client.projects.list({
					org: opts.organization,
					cursor: pageParam ?? undefined,
					limit: RECORDS_PER_PAGE,
				});
				return data;
			},
			getNextPageParam: (lastPage) => {
				if (lastPage.projects.length < RECORDS_PER_PAGE) {
					return undefined;
				}
				return lastPage.pagination.cursor;
			},
			select: (data) => data.pages.flatMap((page) => page.projects),
		});

	const projectQueryOptions = (opts: {
		project: string;
		organization: string;
	}) =>
		queryOptions({
			queryKey: [opts, "project"],
			queryFn: async () => {
				const data = await client.projects.get(opts.project, {
					org: opts.organization,
				});
				return data.project;
			},
			enabled: !!opts.project,
			...no404Retry(),
		});

	const namespaceQueryOptions = (opts: {
		namespace: string;
		organization: string;
		project: string;
	}) => {
		return queryOptions({
			queryKey: [opts, "namespace"],
			queryFn: async () => {
				const data = await client.namespaces.get(
					opts.project,
					opts.namespace,
					{
						org: opts.organization,
					},
				);
				return data.namespace;
			},
			...no404Retry(),
		});
	};

	const organizationsQueryOptions = () =>
		infiniteQueryOptions({
			queryKey: ["organizations"],
			initialPageParam: undefined as number | undefined,
			queryFn: async ({ pageParam }) => {
				if (!clerk.user) {
					throw new Error("No user logged in");
				}
				return clerk.user.getOrganizationMemberships({
					initialPage: pageParam,
					pageSize: RECORDS_PER_PAGE,
				});
			},
			getNextPageParam: (lastPage, allPages) => {
				if (lastPage.data.length < RECORDS_PER_PAGE) {
					return undefined;
				}
				return allPages.reduce(
					(prev, cur) => prev + cur.data.length,
					0,
				);
			},
			select: (data) => data.pages.flatMap((page) => page.data),
		});

	const createProjectMutationOptions = () =>
		mutationOptions({
			mutationKey: ["projects", "create"],
			mutationFn: async (data: {
				displayName: string;
				organization: string;
			}) => {
				const response = await client.projects.create({
					displayName: data.displayName,
					org: data.organization,
				});

				return response;
			},
		});

	const billingProjectSubscriptionUpdateSessionQueryOptions = ({
		project,
		organization,
	}: {
		project: string;
		organization: string;
	}) =>
		queryOptions({
			queryKey: [
				{ organization, project },
				"subscription",
				"update-session",
			],
			queryFn: async () => {
				const response =
					await client.billing.createSubscriptionUpdateSession(
						project,
						{
							org: organization,
						},
					);
				return response.url;
			},
		});

	const projectMetricsQueryOptions = (opts: {
		organization: string;
		project: string;
		name:
			| Rivet.namespaces.MetricsGetRequestNameItem
			| Rivet.namespaces.MetricsGetRequestNameItem[];
		startAt?: string;
		endAt?: string;
		resolution?: number;
	}) =>
		queryOptions({
			queryKey: [opts, "metrics"],
			queryFn: async () => {
				const data = await client.projects.metrics.get(opts.project, {
					name: opts.name,
					org: opts.organization,
					startAt: opts.startAt,
					endAt: opts.endAt,
					resolution: opts.resolution,
				});
				return data;
			},
		});

	const projectLatestMetricsQueryOptions = (opts: {
		organization: string;
		project: string;
		name:
			| Rivet.namespaces.MetricsGetRequestNameItem
			| Rivet.namespaces.MetricsGetRequestNameItem[];
		endAt?: string;
	}) =>
		queryOptions({
			queryKey: [opts, "latest-metrics"],
			queryFn: async () => {
				const data = await client.projects.metrics.getLatest(
					opts.project,
					{
						name: opts.name,
						org: opts.organization,
						endAt: opts.endAt,
					},
				);
				const transformed = data.name.map((name, index) => ({
					name: name as Rivet.namespaces.MetricsGetRequestNameItem,
					ts: data.ts[index],
					value: BigInt(String(data.value[index])),
				}));
				return transformed;
			},
		});

	const namespaceMetricsQueryOptions = (opts: {
		organization: string;
		project: string;
		namespace: string;
		name:
			| Rivet.namespaces.MetricsGetRequestNameItem
			| Rivet.namespaces.MetricsGetRequestNameItem[];
		startAt?: string;
		endAt?: string;
		resolution?: number;
	}) =>
		queryOptions({
			queryKey: [opts, "metrics"],
			queryFn: async () => {
				const data = await client.namespaces.metrics.get(
					opts.project,
					opts.namespace,
					{
						name: opts.name,
						org: opts.organization,
						startAt: opts.startAt,
						endAt: opts.endAt,
						resolution: opts.resolution,
					},
				);
				return data;
			},
		});

	const namespaceLatestMetricsQueryOptions = (opts: {
		organization: string;
		project: string;
		namespace: string;
		name:
			| Rivet.namespaces.MetricsGetRequestNameItem
			| Rivet.namespaces.MetricsGetRequestNameItem[];
		endAt?: string;
	}) =>
		queryOptions({
			queryKey: [opts, "latest-metrics"],
			queryFn: async () => {
				const data = await client.namespaces.metrics.getLatest(
					opts.project,
					opts.namespace,
					{
						name: opts.name,
						org: opts.organization,
						endAt: opts.endAt,
					},
				);
				const transformed = data.name.map((name, index) => ({
					name: name as Rivet.namespaces.MetricsGetRequestNameItem,
					ts: data.ts[index],
					value: BigInt(String(data.value[index])),
				}));
				return transformed;
			},
		});

	return {
		...parent,
		client,
		organization,
		organizationsQueryOptions,
		orgProjectNamespacesQueryOptions,
		currentOrgProjectNamespacesQueryOptions: (opts: {
			project: string;
		}) => {
			return orgProjectNamespacesQueryOptions({
				organization,
				project: opts.project,
			});
		},
		projectsQueryOptions,
		currentOrgProjectsQueryOptions: () => {
			return projectsQueryOptions({ organization });
		},
		currentOrgProjectQueryOptions: (opts: { project: string }) => {
			return projectQueryOptions({ organization, project: opts.project });
		},
		currentOrgProjectNamespaceQueryOptions(opts: {
			project: string;
			namespace: string;
		}) {
			return namespaceQueryOptions({
				organization,
				project: opts.project,
				namespace: opts.namespace,
			});
		},
		createProjectMutationOptions,
		currentOrgCreateProjectMutationOptions() {
			return mutationOptions({
				mutationKey: [{ organization }, "projects", "create"],
				mutationFn: async (data: { displayName: string }) => {
					const response = await client.projects.create({
						displayName: data.displayName,
						org: organization,
					});

					return response;
				},
			});
		},
		billingProjectSubscriptionUpdateSessionQueryOptions,
		currentOrgBillingProjectSubscriptionUpdateSessionQueryOptions(opts: {
			project: string;
		}) {
			return billingProjectSubscriptionUpdateSessionQueryOptions({
				organization,
				project: opts.project,
			});
		},
		currentOrganizationBillingDetailsQueryOptions({
			project,
		}: {
			project: string;
		}) {
			return parent.billingDetailsQueryOptions({
				organization,
				project,
			});
		},
		currentOrganizationProjectMetricsQueryOptions(
			opts: Omit<
				Parameters<typeof projectMetricsQueryOptions>[0],
				"organization"
			>,
		) {
			return projectMetricsQueryOptions({
				organization,
				...opts,
			});
		},
		currentOrganizationProjectLatestMetricsQueryOptions(
			opts: Omit<
				Parameters<typeof projectLatestMetricsQueryOptions>[0],
				"organization"
			>,
		) {
			return projectLatestMetricsQueryOptions({
				organization,
				...opts,
			});
		},
		currentOrganizationNamespaceMetricsQueryOptions(
			opts: Omit<
				Parameters<typeof namespaceMetricsQueryOptions>[0],
				"organization"
			>,
		) {
			return namespaceMetricsQueryOptions({
				organization,
				...opts,
			});
		},
		currentOrganizationNamespaceLatestMetricsQueryOptions(
			opts: Omit<
				Parameters<typeof namespaceLatestMetricsQueryOptions>[0],
				"organization"
			>,
		) {
			return namespaceLatestMetricsQueryOptions({
				organization,
				...opts,
			});
		},
	};
};

export const createProjectContext = ({
	client,
	organization,
	project,
	...parent
}: {
	client: RivetClient;
	organization: string;
	project: string;
} & ReturnType<typeof createOrganizationContext> &
	ReturnType<typeof createGlobalContext>) => {
	return {
		...parent,
		client,
		organization,
		project,
		createNamespaceMutationOptions(opts: {
			onSuccess?: (data: Namespace) => void;
		}) {
			return {
				...opts,
				mutationKey: ["namespaces"],
				mutationFn: async (data: CreateNamespace) => {
					const response = await client.namespaces.create(project, {
						displayName: data.displayName,
						org: organization,
					});
					return {
						id: response.namespace.id,
						name: response.namespace.name,
						displayName: response.namespace.displayName,
						createdAt: new Date(
							response.namespace.createdAt,
						).toISOString(),
					};
				},
			};
		},
		currentProjectQueryOptions: () => {
			return parent.currentOrgProjectQueryOptions({
				project,
			});
		},
		currentProjectNamespacesQueryOptions: () => {
			return parent.orgProjectNamespacesQueryOptions({
				organization,
				project,
			});
		},
		namespacesQueryOptions() {
			return parent.orgProjectNamespacesQueryOptions({
				organization,
				project,
			});
		},
		currentProjectNamespaceQueryOptions(opts: { namespace: string }) {
			return parent.currentOrgProjectNamespaceQueryOptions({
				project,
				namespace: opts.namespace,
			});
		},
		currentProjectBillingDetailsQueryOptions() {
			return parent.currentOrganizationBillingDetailsQueryOptions({
				project,
			});
		},
		changeCurrentProjectBillingPlanMutationOptions() {
			return {
				mutationKey: [{ organization, project }, "billing"],
				mutationFn: async (
					data: Rivet.BillingSetPlanRequest & {
						__from?: Rivet.BillingPlan;
					},
				) => {
					const response = await client.billing.setPlan(project, {
						plan: data.plan,
						org: organization,
					});
					return response;
				},
			};
		},
		accessTokenQueryOptions({ namespace }: { namespace: string }) {
			return queryOptions({
				staleTime: 15 * 60 * 1000, // 15 minutes
				gcTime: 15 * 60 * 1000, // 15 minutes
				queryKey: [
					{ organization, project, namespace },
					"access-token",
				],
				queryFn: async () => {
					const response = await client.namespaces.createAccessToken(
						project,
						namespace,
						{ org: organization },
					);
					return response;
				},
			});
		},
		// API Token methods
		apiTokensQueryOptions() {
			return queryOptions({
				queryKey: [{ organization, project }, "api-tokens"],
				queryFn: async () => {
					const response = await client.apiTokens.list(project, {
						org: organization,
					});
					return response;
				},
			});
		},
		createApiTokenMutationOptions(opts?: {
			onSuccess?: (data: Rivet.ApiTokensCreateRequest) => void;
		}) {
			return {
				mutationKey: [
					{ organization, project },
					"api-tokens",
					"create",
				],
				mutationFn: async (data: {
					name: string;
					expiresAt?: string;
				}) => {
					const response = await client.apiTokens.create(project, {
						name: data.name,
						expiresAt: data.expiresAt,
						org: organization,
					});
					return response;
				},
				onSuccess: opts?.onSuccess,
			};
		},
		revokeApiTokenMutationOptions(opts?: { onSuccess?: () => void }) {
			return {
				mutationKey: [
					{ organization, project },
					"api-tokens",
					"revoke",
				],
				mutationFn: async (data: { apiTokenId: string }) => {
					const response = await client.apiTokens.revoke(
						project,
						data.apiTokenId,
						{ org: organization },
					);
					return response;
				},
				onSuccess: opts?.onSuccess,
			};
		},
		currentProjectBillingSubscriptionUpdateSessionQueryOptions() {
			return parent.billingProjectSubscriptionUpdateSessionQueryOptions({
				organization,
				project,
			});
		},
		currentProjectMetricsQueryOptions(
			opts: Omit<
				Parameters<
					typeof parent.currentOrganizationProjectMetricsQueryOptions
				>[0],
				"project"
			>,
		) {
			return parent.currentOrganizationProjectMetricsQueryOptions({
				project,
				...opts,
			});
		},
		currentProjectLatestMetricsQueryOptions(
			opts: Omit<
				Parameters<
					typeof parent.currentOrganizationProjectLatestMetricsQueryOptions
				>[0],
				"project"
			>,
		) {
			return parent.currentOrganizationProjectLatestMetricsQueryOptions({
				project,
				...opts,
			});
		},
		currentProjectNamespaceMetricsQueryOptions(
			opts: Omit<
				Parameters<
					typeof parent.currentOrganizationNamespaceMetricsQueryOptions
				>[0],
				"project"
			>,
		) {
			return parent.currentOrganizationNamespaceMetricsQueryOptions({
				project,
				...opts,
			});
		},
		currentProjectNamespaceLatestMetricsQueryOptions(
			opts: Omit<
				Parameters<
					typeof parent.currentOrganizationNamespaceLatestMetricsQueryOptions
				>[0],
				"project"
			>,
		) {
			return parent.currentOrganizationNamespaceLatestMetricsQueryOptions(
				{
					project,
					...opts,
				},
			);
		},
	};
};

export const createNamespaceContext = ({
	namespace,
	engineNamespaceName,
	engineNamespaceId,
	...parent
}: {
	namespace: string;
	engineNamespaceName: string;
	engineNamespaceId: string;
} & ReturnType<typeof createProjectContext> &
	ReturnType<typeof createOrganizationContext> &
	ReturnType<typeof createGlobalContext>) => {
	const token = async () => {
		const response = await queryClient.fetchQuery(
			parent.accessTokenQueryOptions({ namespace }),
		);

		return response.token;
	};
	return {
		...parent,
		...createEngineNamespaceContext({
			...parent,
			namespace: engineNamespaceName,
			engineToken: token,
			client: createEngineClient(cloudEnv().VITE_APP_API_URL, {
				token,
			}),
		}),
		namespaceQueryOptions() {
			return parent.currentProjectNamespaceQueryOptions({ namespace });
		},
		currentNamespaceAccessTokenQueryOptions() {
			return parent.accessTokenQueryOptions({ namespace });
		},
		engineAdminTokenQueryOptions(): UseQueryOptions<string> {
			return queryOptions({
				staleTime: 5 * 60 * 1000, // 5 minutes
				gcTime: 5 * 60 * 1000, // 5 minutes
				queryKey: [
					{
						namespace,
						project: parent.project,
						organization: parent.organization,
					},
					"tokens",
					"engine-admin",
				] as QueryKey,
				queryFn: async () => {
					const f = parent.client.namespaces.createSecretToken(
						parent.project,
						namespace,
						{ org: parent.organization },
					);
					const t = await f;
					return t.token;
				},
			});
		},
		publishableTokenQueryOptions() {
			return queryOptions({
				staleTime: 5 * 60 * 1000, // 5 minutes
				gcTime: 5 * 60 * 1000, // 5 minutes
				queryKey: [
					{
						namespace,
						project: parent.project,
						organization: parent.organization,
					},
					"tokens",
					"publishable",
				],
				queryFn: async () => {
					const f = parent.client.namespaces.createPublishableToken(
						parent.project,
						namespace,
						{ org: parent.organization },
					);
					const t = await f;
					return t.token as string;
				},
			});
		},
		currentNamespaceQueryOptions() {
			return parent.currentProjectNamespaceQueryOptions({ namespace });
		},
		currentNamespaceMetricsQueryOptions(
			opts: Omit<
				Parameters<
					typeof parent.currentProjectNamespaceMetricsQueryOptions
				>[0],
				"namespace"
			>,
		) {
			return parent.currentProjectNamespaceMetricsQueryOptions({
				namespace,
				...opts,
			});
		},
		currentNamespaceLatestMetricsQueryOptions(
			opts: Omit<
				Parameters<
					typeof parent.currentProjectNamespaceLatestMetricsQueryOptions
				>[0],
				"namespace"
			>,
		) {
			return parent.currentProjectNamespaceLatestMetricsQueryOptions({
				namespace,
				...opts,
			});
		},
	};
};
