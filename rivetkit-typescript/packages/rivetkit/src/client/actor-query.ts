import type { Context as HonoContext } from "hono";
import * as errors from "@/actor/errors";
import { stringifyError } from "@/common/utils";
import type { ManagerDriver } from "@/driver-helpers/mod";
import type { ActorQuery } from "@/manager/protocol/query";
import { ActorSchedulingError } from "./errors";
import { logger } from "./log";

/**
 * Query the manager driver to get or create a actor based on the provided query
 */
export async function queryActor(
	c: HonoContext | undefined,
	query: ActorQuery,
	managerDriver: ManagerDriver,
): Promise<{ actorId: string }> {
	logger().debug({ msg: "querying actor", query: JSON.stringify(query) });
	let actorOutput: { actorId: string };
	if ("getForId" in query) {
		const output = await managerDriver.getForId({
			c,
			name: query.getForId.name,
			actorId: query.getForId.actorId,
		});
		if (!output) throw new errors.ActorNotFound(query.getForId.actorId);
		actorOutput = output;
	} else if ("getForKey" in query) {
		const existingActor = await managerDriver.getWithKey({
			c,
			name: query.getForKey.name,
			key: query.getForKey.key,
		});
		if (!existingActor) {
			throw new errors.ActorNotFound(
				`${query.getForKey.name}:${JSON.stringify(query.getForKey.key)}`,
			);
		}
		actorOutput = existingActor;
	} else if ("getOrCreateForKey" in query) {
		const getOrCreateOutput = await managerDriver.getOrCreateWithKey({
			c,
			name: query.getOrCreateForKey.name,
			key: query.getOrCreateForKey.key,
			input: query.getOrCreateForKey.input,
			region: query.getOrCreateForKey.region,
		});
		actorOutput = {
			actorId: getOrCreateOutput.actorId,
		};
	} else if ("create" in query) {
		const createOutput = await managerDriver.createActor({
			c,
			name: query.create.name,
			key: query.create.key,
			input: query.create.input,
			region: query.create.region,
		});
		actorOutput = {
			actorId: createOutput.actorId,
		};
	} else {
		throw new errors.InvalidRequest("Invalid query format");
	}

	logger().debug({ msg: "actor query result", actorId: actorOutput.actorId });
	return { actorId: actorOutput.actorId };
}

/**
 * Extract the actor name from a query.
 */
export function getActorNameFromQuery(query: ActorQuery): string {
	if ("getForId" in query) return query.getForId.name;
	if ("getForKey" in query) return query.getForKey.name;
	if ("getOrCreateForKey" in query) return query.getOrCreateForKey.name;
	if ("create" in query) return query.create.name;
	throw new errors.InvalidRequest("Invalid query format");
}

/**
 * Fetch actor details and check for scheduling errors.
 */
export async function checkForSchedulingError(
	group: string,
	code: string,
	actorId: string,
	query: ActorQuery,
	driver: ManagerDriver,
): Promise<ActorSchedulingError | null> {
	const name = getActorNameFromQuery(query);

	try {
		const actor = await driver.getForId({ name, actorId });

		if (actor?.error) {
			logger().info({
				msg: "found actor scheduling error",
				actorId,
				error: actor.error,
			});
			return new ActorSchedulingError(group, code, actorId, actor.error);
		}
	} catch (err) {
		logger().warn({
			msg: "failed to fetch actor details for scheduling error check",
			actorId,
			error: stringifyError(err),
		});
	}

	return null;
}
