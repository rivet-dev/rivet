// @ts-types="../../common/dist/network.d.ts"
import { PORT_NAME } from "@rivet-gg/actor-common/network";
// @ts-types="../../common/dist/utils.d.ts"
import { assertUnreachable } from "@rivet-gg/actor-common/utils";
// @ts-types="../../common/dist/utils.d.ts"
import type {
	ActorTags,
	BuildTags,
	RivetEnvironment,
} from "@rivet-gg/actor-common/utils";
import type { Rivet, RivetClient } from "@rivet-gg/api";
// @ts-types="../../manager-protocol/dist/query.d.ts"
import type {
	ActorQuery,
	CreateRequest,
} from "@rivet-gg/manager-protocol/query";
import { logger } from "./log";

export async function queryActor(
	client: RivetClient,
	environment: RivetEnvironment,
	query: ActorQuery,
): Promise<Rivet.actor.Actor> {
	logger().debug("query", { query });
	if ("getForId" in query) {
		// Get actor
		const res = await client.actor.get(query.getForId.actorId, environment);

		// Validate actor
		if ((res.actor.tags as ActorTags).access !== "public") {
			// TODO: Throw 404 that matches the 404 from Fern if the actor is not found
			throw new Error(
				`Actor with ID ${query.getForId.actorId} is private`,
			);
		}
		if (res.actor.destroyedAt) {
			throw new Error(
				`Actor with ID ${query.getForId.actorId} already destroyed`,
			);
		}

		return res.actor;
	}
	if ("getOrCreateForTags" in query) {
		const tags = query.getOrCreateForTags.tags;
		if (!tags) throw new Error("Must define tags in getOrCreateForTags");
		const existingActor = await getWithTags(
			client,
			environment,
			tags as ActorTags,
		);
		if (existingActor) {
			// Actor exists
			return existingActor;
		}

		if (query.getOrCreateForTags.create) {
			// Create if needed
			return await createActor(
				client,
				environment,
				query.getOrCreateForTags.create,
			);
		}
		// Creation disabled
		throw new Error("Actor not found with tags or is private.");
	}
	if ("create" in query) {
		return await createActor(client, environment, query.create);
	}
	assertUnreachable(query);
}

async function getWithTags(
	client: RivetClient,
	environment: RivetEnvironment,
	tags: ActorTags,
): Promise<Rivet.actor.Actor | undefined> {
	const req = {
		tagsJson: JSON.stringify({
			...tags,
			access: "public",
		}),
		...environment,
	};
	let { actors } = await client.actor.list(req);

	// TODO(RVT-4248): Don't return actors that aren't networkable yet
	actors = actors.filter((a) => {
		// This should never be triggered. This assertion will leak if private actors exist if it's ever triggered.
		if ((a.tags as ActorTags).access !== "public") {
			throw new Error("unreachable: actor tags not public");
		}

		for (const portName in a.network.ports) {
			const port = a.network.ports[portName];
			if (!port.hostname || !port.port) return false;
		}
		return true;
	});

	if (actors.length === 0) {
		return undefined;
	}

	// Make the chosen actor consistent
	if (actors.length > 1) {
		actors.sort((a, b) => a.id.localeCompare(b.id));
	}

	return actors[0];
}

async function createActor(
	client: RivetClient,
	environment: RivetEnvironment,
	createRequest: CreateRequest,
): Promise<Rivet.actor.Actor> {
	// Verify build access
	const build = await getBuildWithTags(client, environment, {
		name: createRequest.tags.name,
		current: "true",
		access: "public",
	});
	if (!build) throw new Error("Build not found with tags or is private");

	// Create actor
	const req: Rivet.actor.CreateActorRequestQuery = {
		...environment,
		body: {
			tags: {
				...createRequest.tags,
				access: "public",
			},
			build: build.id,
			region: createRequest.region,
			network: {
				ports: {
					[PORT_NAME]: {
						protocol: "https",
						routing: { guard: {} },
					},
				},
			},
		},
	};
	logger().info("creating actor", { ...req });
	const { actor } = await client.actor.create(req);
	return actor;
}

async function getBuildWithTags(
	client: RivetClient,
	environment: RivetEnvironment,
	buildTags: BuildTags,
): Promise<Rivet.actor.Build | undefined> {
	const req = {
		tagsJson: JSON.stringify(buildTags),
		...environment,
	};
	let { builds } = await client.actor.builds.list(req);

	builds = builds.filter((b) => {
		// Filter out private builds
		if ((b.tags as BuildTags).access !== "public") return false;

		return true;
	});

	if (builds.length === 0) {
		return undefined;
	}
	if (builds.length > 1) {
		builds.sort((a, b) => a.id.localeCompare(b.id));
	}

	return builds[0];
}
