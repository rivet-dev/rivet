import type { Clerk } from "@clerk/clerk-js";
import { type Rivet, RivetClient } from "@rivet-gg/cloud";
import { infiniteQueryOptions, queryOptions } from "@tanstack/react-query";
import { cloudEnv } from "@/lib/env";
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
	});
}

export const createGlobalContext = ({ clerk }: { clerk: Clerk }) => {
	return {
		client: createClient({ clerk }),
		organizationQueryOptions(opts: { org: string }) {
			return queryOptions({
				queryKey: ["organization", opts.org],
				queryFn: async () => {
					return clerk.getOrganization(opts.org);
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
				return data;
			},
			enabled: !!opts.project,
		});

	return {
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
			return queryOptions({
				queryKey: [opts, "namespace"],
				queryFn: async ({ signal: abortSignal }) => {
					const data = await client.namespaces.get(
						opts.project,
						opts.namespace,
						{
							org: organization,
						},
						{ abortSignal },
					);
					return data;
				},
				enabled: !!opts.namespace,
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
						name: data.nameId,
						organizationId: organization,
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
		createNamespaceMutationOptions(opts: {
			onSuccess?: (data: Namespace) => void;
		}) {
			return {
				...opts,
				mutationKey: ["namespaces"],
				mutationFn: async (data: CreateNamespace) => {
					const response = await client.namespaces.create(project, {
						name: data.name,
						displayName: data.displayName,
						org: organization,
					});
					return response;
				},
			};
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
	};
};

export const createNamespaceContext = ({
	namespace,
	...parent
}: { namespace: string } & ReturnType<typeof createProjectContext> &
	ReturnType<typeof createOrganizationContext> &
	ReturnType<typeof createGlobalContext>) => {
	return {
		...createEngineNamespaceContext({
			namespace,
			...parent,
			client: createEngineClient(),
		}),
	};
};
