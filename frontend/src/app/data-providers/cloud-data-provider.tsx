import type { Clerk } from "@clerk/clerk-js";
import { type Rivet, RivetClient } from "@rivet-gg/cloud";
import { type FetchFunction, fetcher } from "@rivetkit/engine-api-full/core";
import { infiniteQueryOptions, queryOptions } from "@tanstack/react-query";
import { cloudEnv } from "@/lib/env";
import { queryClient } from "@/queries/global";
import { RECORDS_PER_PAGE } from "./default-data-provider";
import {
	type CreateNamespace,
	createClient as createEngineClient,
	createNamespaceContext as createEngineNamespaceContext,
	type Namespace,
} from "./engine-data-provider";

function createClient({ clerk }: { clerk: Clerk }) {
	return new RivetClient({
		baseUrl: () => cloudEnv().VITE_APP_CLOUD_API_URL,
		environment: "",
		token: async () => {
			return (await clerk.session?.getToken()) || "";
		},
		fetcher: async (args) => {
			if (args.headers) {
				delete args.headers["X-Fern-Language"];
				delete args.headers["X-Fern-Runtime"];
				delete args.headers["X-Fern-Runtime-Version"];
			}
			return await fetcher(args);
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
	};
};

export const createOrganizationContext = ({
	client,
	organization,
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
			queryFn: async ({ pageParam, signal: abortSignal }) => {
				const data = await client.namespaces.list(
					opts.project,
					{
						org: opts.organization,
						limit: RECORDS_PER_PAGE,
						cursor: pageParam ?? undefined,
					},
					{ abortSignal },
				);
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
				if (lastPage.namespaces.length < RECORDS_PER_PAGE) {
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
			queryFn: async ({ signal: abortSignal, pageParam }) => {
				const data = await client.projects.list(
					{
						org: opts.organization,
						cursor: pageParam ?? undefined,
						limit: RECORDS_PER_PAGE,
					},
					{
						abortSignal,
					},
				);
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
			queryFn: async ({ signal: abortSignal }) => {
				const data = await client.projects.get(
					opts.project,
					{
						org: opts.organization,
					},
					{ abortSignal },
				);
				return data.project;
			},
			enabled: !!opts.project,
		});

	const namespaceQueryOptions = (opts: {
		namespace: string;
		organization: string;
		project: string;
	}) => {
		return queryOptions({
			queryKey: [opts, "namespace"],
			queryFn: async ({ signal: abortSignal }) => {
				const data = await client.namespaces.get(
					opts.project,
					opts.namespace,
					{
						org: opts.organization,
					},
					{ abortSignal },
				);
				return data.namespace;
			},
		});
	};

	return {
		organization,
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
		currentOrgCreateProjectMutationOptions({
			onSuccess,
		}: {
			onSuccess?: (data: Rivet.Project) => void;
		} = {}) {
			return {
				mutationKey: ["projects"],
				mutationFn: async (data: {
					displayName: string;
					nameId: string;
				}) => {
					const response = await client.projects.create({
						displayName: data.displayName,
						org: organization,
					});

					return response;
				},
				onSuccess,
			};
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
			return queryOptions({
				queryKey: [{ organization, project }, "billing-details"],
				queryFn: async ({ signal: abortSignal }) => {
					const response = await client.billing.details(
						project,
						{ org: organization },
						{ abortSignal },
					);
					return response;
				},
			});
		},
		changeCurrentProjectBillingPlanMutationOptions() {
			return {
				mutationKey: [{ organization, project }, "billing"],
				mutationFn: async (data: Rivet.BillingSetPlanRequest) => {
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
				queryFn: async ({ signal: abortSignal }) => {
					const response = await client.namespaces.createAccessToken(
						project,
						namespace,
						{ org: organization },
						{ abortSignal },
					);
					return response;
				},
			});
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
	return {
		...createEngineNamespaceContext({
			...parent,
			namespace: engineNamespaceName,
			namespaceId: engineNamespaceId,
			client: createEngineClient(cloudEnv().VITE_APP_CLOUD_ENGINE_URL, {
				token: async () => {
					const response = await queryClient.fetchQuery(
						parent.accessTokenQueryOptions({ namespace }),
					);

					return response.token;
				},
			}),
		}),
		namespaceQueryOptions() {
			return parent.currentProjectNamespaceQueryOptions({ namespace });
		},
		connectRunnerTokenQueryOptions() {
			return queryOptions({
				staleTime: 5 * 60 * 1000, // 5 minutes
				gcTime: 5 * 60 * 1000, // 5 minutes
				queryKey: [
					{
						namespace,
						project: parent.project,
						organization: parent.organization,
					},
					"runners",
					"connect",
				],
				queryFn: async () => {
					const f = parent.client.namespaces.createPublishableToken(
						parent.project,
						namespace,
						{ org: parent.organization },
					);
					const t = await f;
					return t.token;
				},
			});
		},
	};
};
