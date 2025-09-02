import {
	createManagerInspectorClient,
	type Actor as InspectorActor,
} from "@rivetkit/core/inspector";
import { infiniteQueryOptions } from "@tanstack/react-query";
import type { Actor, ActorId, ManagerContext } from "@/components/actors";
import { createDefaultManagerContext } from "@/components/actors/manager-context";
import { ensureTrailingSlash } from "@/lib/utils";
import { queryClient } from "./global";

export const createClient = (url: string, token: string) => {
	const newUrl = new URL(url);
	if (!newUrl.pathname.endsWith("registry/inspect")) {
		if (!newUrl.pathname.endsWith("registry")) {
			newUrl.pathname = `${ensureTrailingSlash(newUrl.pathname)}registry`;
		}
		if (!newUrl.pathname.endsWith("inspect")) {
			newUrl.pathname = `${ensureTrailingSlash(newUrl.pathname)}inspect`;
		}
	}

	return createManagerInspectorClient(newUrl.href, {
		headers: { Authorization: `Bearer ${token}` },
	});
};

export const createInspectorManagerContext = ({
	url,
	token,
}: {
	url: string;
	token: string;
}) => {
	const client = createClient(url, token);

	const def = createDefaultManagerContext();

	return {
		...def,
		endpoint: url,
		features: {
			canCreateActors: true,
			canDeleteActors: false,
		},
		managerStatusQueryOptions() {
			return {
				...def.managerStatusQueryOptions(),
				enabled: true,
				queryFn: async ({ signal }) => {
					const status = await client.ping.$get({ signal });
					if (!status.ok) {
						throw new Error("Failed to fetch manager status");
					}
					return true;
				},
			};
		},
		regionsQueryOptions() {
			return infiniteQueryOptions({
				...def.regionsQueryOptions(),
				enabled: true,
				queryFn: async () => {
					return {
						regions: [{ id: "local", name: "Local" }],
						pagination: {},
					};
				},
			});
		},
		actorQueryOptions(actorId) {
			return {
				...def.actorQueryOptions(actorId),
				enabled: true,
				queryFn: async ({ signal }) => {
					const response = await client.actor[":id"].$get({
						param: { id: actorId },
						// @ts-expect-error
						signal,
					});
					if (!response.ok) {
						throw response;
					}
					return transformActor(await response.json());
				},
			};
		},
		actorsQueryOptions(opts) {
			return infiniteQueryOptions({
				...def.actorsQueryOptions(opts),
				enabled: true,
				initialPageParam: undefined,
				queryFn: async ({ signal, pageParam }) => {
					const actors = await client.actors.$get({
						query: { cursor: pageParam, limit: 10 },
						signal,
					});
					if (!actors.ok) {
						throw new Error("Failed to fetch actors");
					}
					const response = await actors.json();

					return {
						actors: response.map((actor) => transformActor(actor)),
						pagination: {
							cursor:
								response.length === 10
									? response[9].id
									: undefined,
						},
					};
				},
			});
		},
		buildsQueryOptions() {
			return infiniteQueryOptions({
				...def.buildsQueryOptions(),
				enabled: true,
				initialPageParam: undefined,
				queryFn: async ({ signal, pageParam }) => {
					const builds = await client.builds.$get({
						query: { cursor: pageParam, limit: 10 },
						signal,
					});
					if (!builds.ok) {
						throw new Error("Failed to fetch builds");
					}
					const response = await builds.json();

					return {
						builds: response.map((build) => ({
							id: build.name,
							name: build.name,
						})),
						pagination: {
							cursor:
								response.length === 10
									? response[9].name
									: undefined,
						},
					};
				},
			});
		},
		createActorMutationOptions() {
			return {
				...def.createActorMutationOptions(),
				mutationFn: async (data) => {
					const response = await client.actors.$post({
						json: {
							key: [data.key],
							name: data.name,
							input: data.input,
						},
					});
					if (!response.ok) {
						throw new Error("Failed to create actor");
					}
					const json = await response.json();
					if (!json?.id) {
						throw new Error("Failed to create actor");
					}
					return json.id;
				},
			};
		},
	} satisfies ManagerContext;
};

function transformActor(a: InspectorActor): Actor {
	return {
		id: a.id as ActorId,
		name: a.name,
		key: a.key.join(" "),
		createdAt: a.createdAt
			? new Date(a.createdAt).toISOString()
			: undefined,
		destroyedAt: a.destroyedAt
			? new Date(a.destroyedAt).toISOString()
			: undefined,
		startedAt: a.createdAt
			? new Date(a.createdAt).toISOString()
			: undefined,
		features: a.features,
	};
}
