import type { AuthContext } from "@/domains/auth/contexts/auth";
import {
	bootstrapQueryOptions,
	clusterQueryOptions,
} from "@/domains/auth/queries/bootstrap";
import {
	projectByIdQueryOptions,
	projectEnvironmentQueryOptions,
	projectQueryOptions,
	projectsByGroupQueryOptions,
} from "@/domains/project/queries";
import { type QueryClient, useSuspenseQuery } from "@tanstack/react-query";
import {
	type ParsedLocation,
	notFound,
	redirect,
} from "@tanstack/react-router";
import type { PropsWithChildren } from "react";
import { ls } from "./ls";
import { isUuid } from "./utils";

export function GuardEnterprise({ children }: PropsWithChildren) {
	const { data: cluster } = useSuspenseQuery(clusterQueryOptions());

	if (cluster === "enterprise") {
		return children;
	}

	return null;
}

export async function guardEnterprise({
	queryClient,
}: { queryClient: QueryClient }) {
	const bootstrap = await queryClient.fetchQuery(bootstrapQueryOptions());

	if (bootstrap.cluster === "oss") {
		throw notFound();
	}
}

export async function guardOssNewbie({
	queryClient,
	auth,
}: { queryClient: QueryClient; auth: AuthContext }) {
	const { cluster } = await queryClient.fetchQuery(bootstrapQueryOptions());

	const { games: projects, groups } = await queryClient.fetchQuery(
		projectsByGroupQueryOptions(),
	);

	if (cluster === "oss" && projects.length === 1) {
		const {
			game: { namespaces },
		} = await queryClient.fetchQuery(
			projectQueryOptions(projects[0].gameId),
		);

		// In case the project has no namespaces, or we failed to fetch the project, redirect to the project page
		if (namespaces.length > 0) {
			throw redirect({
				to: "/projects/$projectNameId/environments/$environmentNameId",
				params: {
					projectNameId: projects[0].nameId,
					environmentNameId: namespaces[0].nameId,
				},
				from: "/",
			});
		}
		throw redirect({
			to: "/projects/$projectNameId",
			params: {
				projectNameId: projects[0].nameId,
			},
			from: "/",
		});
	}

	const lastTeam = ls.recentTeam.get(auth);

	if (lastTeam) {
		throw redirect({
			to: "/teams/$groupId",
			params: { groupId: lastTeam },
			from: "/",
		});
	}

	if (groups.length > 0) {
		throw redirect({
			to: "/teams/$groupId",
			params: { groupId: groups[0].groupId },
			from: "/",
		});
	}
}

export async function guardUuids({
	queryClient,
	projectNameId,
	environmentNameId,
	location,
}: {
	queryClient: QueryClient;
	projectNameId: string;
	environmentNameId: string | undefined;
	location: ParsedLocation;
}) {
	let pathname = location.pathname;

	if (isUuid(projectNameId)) {
		const response = await queryClient.fetchQuery(
			projectsByGroupQueryOptions(),
		);
		const project = response?.games.find((p) => p.gameId === projectNameId);
		if (project) {
			pathname = pathname.replace(projectNameId, project.nameId);
		}
	}

	if (environmentNameId && isUuid(environmentNameId)) {
		const { games: projects } = await queryClient.fetchQuery(
			projectByIdQueryOptions(projectNameId),
		);

		const envProject = projects.find((p) => p.nameId === projectNameId);

		if (!envProject) {
			// bail out if we can't find the project
			return;
		}

		const { namespace: environment } = await queryClient.fetchQuery(
			projectEnvironmentQueryOptions({
				projectId: envProject.gameId,
				environmentId: environmentNameId,
			}),
		);

		if (!environment) {
			// bail out if we can't find the environment
			return;
		}

		pathname = pathname.replace(environmentNameId, environment.nameId);
	}

	if (pathname !== location.pathname) {
		throw redirect({
			to: pathname,
			replace: true,
		});
	}
}
