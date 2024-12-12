import type { Rivet, RivetClient } from "@rivet-gg/api";
import { PORT_NAME } from "../../common/src/network.ts";
import { assertUnreachable } from "../../common/src/utils.ts";
import type {
	ActorTags,
	BuildTags,
	RivetEnvironment,
} from "../../common/src/utils.ts";
import type {
	ActorQuery,
	CreateRequest,
} from "../../manager-protocol/src/query.ts";

export async function queryActor(
	client: RivetClient,
	environment: RivetEnvironment,
	query: ActorQuery,
): Promise<Rivet.actor.Actor> {
	console.log("Query", query);
	if ("actorId" in query) {
		const res = await client.actor.get(query.actorId.actorId, environment);
		if ((res.actor.tags as ActorTags).access !== "public") {
			throw new Error(`Actor with ID ${query.actorId.actorId} is private`);
		}
		if (res.actor.destroyedAt) {
			throw new Error(
				`Actor with ID ${query.actorId.actorId} already destroyed`,
			);
		}
		return res.actor;
	} else if ("get" in query) {
		const getActor = await getWithTags(client, environment, query.get.tags);
		if (!getActor) throw new Error("Actor not found with tags or is private");
		return getActor;
	} else if ("getOrCreate" in query) {
		const tags = query.getOrCreate.tags;
		if (!tags) throw new Error("Must define tags in getOrCreate");
		const getActor = await getWithTags(client, environment, tags as ActorTags);
		if (getActor) {
			return getActor;
		} else {
			return await createActor(client, environment, query.getOrCreate.create);
		}
	} else if ("create" in query) {
		return await createActor(client, environment, query.create);
	} else {
		assertUnreachable(query);
	}
}

async function getWithTags(
	client: RivetClient,
	environment: RivetEnvironment,
	tags: ActorTags,
): Promise<Rivet.actor.Actor | undefined> {
	const req = {
		tagsJson: JSON.stringify(tags),
		...environment,
	};
	console.log("List request", req);
	let { actors } = await client.actor.list(req);

	// TODO(RVT-4248): Don't return actors that aren't networkable yet
	actors = actors.filter((a) => {
		// Filter out private actors
		if ((a.tags as ActorTags).access !== "public") return false;

		for (const portName in a.network.ports) {
			const port = a.network.ports[portName];
			if (!port.hostname || !port.port) return false;
		}
		return true;
	});

	if (actors.length === 0) {
		return undefined;
	}
	if (actors.length > 1) {
		actors.sort((a, b) => a.id.localeCompare(b.id));
	}

	return actors[0];
}

async function createActor(
	client: RivetClient,
	environment: RivetEnvironment,
	createRequest: CreateRequest = {} satisfies CreateRequest,
): Promise<Rivet.actor.Actor> {
	if (!createRequest.network) {
		createRequest.network = {};
	}
	if (!createRequest.network.ports) {
		createRequest.network.ports = {};
	}
	if (!(PORT_NAME in createRequest.network.ports)) {
		createRequest.network.ports[PORT_NAME] = {
			protocol: "https",
			routing: { guard: {} },
		};
	}

	// Verify build access
	if (createRequest.build) {
		const { build } = await client.actor.builds.get(createRequest.build);

		if (build.tags.access !== "public") {
			throw new Error("Cannot create actor with private build");
		}
	} else if (createRequest.buildTags) {
		const build = await getBuildWithTags(
			client,
			environment,
			createRequest.buildTags as BuildTags,
		);
		if (!build) throw new Error("Build not found with tags or is private");
	}

	const req = {
		...environment,
		body: createRequest,
	};
	console.log("Create actor", req);
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
	console.log("List builds request", req);
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
