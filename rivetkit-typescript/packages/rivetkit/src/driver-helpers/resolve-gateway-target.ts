import { ActorNotFound, InvalidRequest } from "@/actor/errors";
import type { GatewayTarget, ManagerDriver } from "@/manager/driver";

/**
 * Resolves a GatewayTarget to a concrete actor ID string.
 *
 * Shared across all ManagerDriver implementations to avoid duplicating the
 * same query-to-actorId dispatch logic.
 */
export async function resolveGatewayTarget(
	driver: ManagerDriver,
	target: GatewayTarget,
): Promise<string> {
	if ("directId" in target) {
		return target.directId;
	}

	if ("getForKey" in target) {
		const output = await driver.getWithKey({
			name: target.getForKey.name,
			key: target.getForKey.key,
		});
		if (!output) {
			throw new ActorNotFound(
				`${target.getForKey.name}:${JSON.stringify(target.getForKey.key)}`,
			);
		}
		return output.actorId;
	}

	if ("getOrCreateForKey" in target) {
		const output = await driver.getOrCreateWithKey({
			name: target.getOrCreateForKey.name,
			key: target.getOrCreateForKey.key,
			input: target.getOrCreateForKey.input,
			region: target.getOrCreateForKey.region,
		});
		return output.actorId;
	}

	if ("create" in target) {
		const output = await driver.createActor({
			name: target.create.name,
			key: target.create.key,
			input: target.create.input,
			region: target.create.region,
		});
		return output.actorId;
	}

	throw new InvalidRequest("Invalid query format");
}
