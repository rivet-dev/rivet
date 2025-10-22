import { sValidator } from "@hono/standard-validator";
import { Hono } from "hono";
import invariant from "invariant";
import type { CreateInput } from "@/manager/driver";
import { inspectorLogger } from "./log";
import { type Actor, type Builds, CreateActorSchema } from "./protocol/common";

export type ManagerInspectorRouterEnv = {
	Variables: {
		inspector: ManagerInspector;
	};
};

/**
 * Create a router for the Manager Inspector.
 * @internal
 */
export function createManagerInspectorRouter() {
	return new Hono<ManagerInspectorRouterEnv>()
		.get("/ping", (c) => {
			return c.json({ message: "pong" }, 200);
		})
		.get("/actors", async (c) => {
			const limit =
				Number.parseInt(c.req.query("limit") ?? "") || undefined;
			const cursor = c.req.query("cursor") || undefined;

			if (!limit || (limit && limit <= 0)) {
				return c.json("Invalid limit", 400);
			}

			try {
				const actors = await c.var.inspector.accessors.getAllActors({
					limit,
					cursor,
				});
				return c.json(actors, 200);
			} catch (error) {
				inspectorLogger().error({
					msg: "Failed to fetch actors",
					error,
				});
				return c.json("Failed to fetch actors", 500);
			}
		})

		.post("/actors", sValidator("json", CreateActorSchema), async (c) => {
			const actor = await c.var.inspector.accessors.createActor(
				c.req.valid("json"),
			);
			return c.json(actor, 201);
		})
		.get("/builds", async (c) => {
			const builds = await c.var.inspector.accessors.getBuilds();
			return c.json(builds, 200);
		})
		.get("/actor/:id", async (c) => {
			const id = c.req.param("id");
			const actor = await c.var.inspector.accessors.getActorById(id);
			if (!actor) {
				return c.json({ error: "Actor not found" }, 404);
			}
			return c.json(actor, 200);
		})
		.get("/bootstrap", async (c) => {
			const actors = await c.var.inspector.accessors.getAllActors({
				limit: 10,
			});
			return c.json({ actors }, 200);
		});
}

interface ManagerInspectorAccessors {
	getAllActors: (param: {
		cursor?: string;
		limit: number;
	}) => Promise<Actor[]>;
	getActorById: (id: string) => Promise<Actor | null>;
	getBuilds: () => Promise<Builds>;
	createActor: (input: CreateInput) => Promise<Actor | null>;
}

/**
 * Provides a unified interface for inspecting actor external and internal state.
 */
export class ManagerInspector {
	public readonly accessors: ManagerInspectorAccessors;

	constructor(accessors: () => ManagerInspectorAccessors) {
		this.accessors = accessors();
		inspectorLogger().debug({ msg: "Manager Inspector enabled and ready" });
	}
}
