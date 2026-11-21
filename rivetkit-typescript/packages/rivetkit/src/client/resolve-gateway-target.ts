import { actorNotFound, invalidRequest } from "@/actor/errors";
import type {
	EngineControlClient,
	GatewayTarget,
} from "@/engine-client/driver";
import {
	isRetryableCreateRefusal,
	retryOnLifecycleBoundary,
} from "./lifecycle-errors";
import type { RetryBudget } from "./retry-budget";

/**
 * Resolves a GatewayTarget to a concrete actor ID string.
 *
 * Shared across all EngineControlClient implementations to avoid duplicating the
 * same query-to-actorId dispatch logic. The client is the retry owner of a create: one the
 * engine refused before submitting it is sent again after the service's hint, within
 * `opts.budget`.
 */
export async function resolveGatewayTarget(
	driver: EngineControlClient,
	target: GatewayTarget,
	opts?: { budget?: RetryBudget; signal?: AbortSignal },
): Promise<string> {
	const retryCreate = <T>(run: () => Promise<T>) =>
		retryOnLifecycleBoundary(run, {
			signal: opts?.signal,
			budget: opts?.budget,
			isRetryable: isRetryableCreateRefusal,
		});

	if ("directId" in target) {
		return target.directId;
	}

	if ("getForId" in target) {
		return target.getForId.actorId;
	}

	if ("getForKey" in target) {
		const output = await driver.getWithKey({
			name: target.getForKey.name,
			key: target.getForKey.key,
		});
		if (!output) {
			throw actorNotFound(
				`${target.getForKey.name}:${JSON.stringify(target.getForKey.key)}`,
			);
		}
		return output.actorId;
	}

	if ("getOrCreateForKey" in target) {
		const output = await retryCreate(() =>
			driver.getOrCreateWithKey({
				name: target.getOrCreateForKey.name,
				key: target.getOrCreateForKey.key,
				input: target.getOrCreateForKey.input,
				region: target.getOrCreateForKey.region,
				poolName: target.getOrCreateForKey.poolName,
			}),
		);
		return output.actorId;
	}

	if ("create" in target) {
		const output = await retryCreate(() =>
			driver.createActor({
				name: target.create.name,
				key: target.create.key,
				input: target.create.input,
				region: target.create.region,
				poolName: target.create.poolName,
			}),
		);
		return output.actorId;
	}

	throw invalidRequest("Invalid query format");
}
