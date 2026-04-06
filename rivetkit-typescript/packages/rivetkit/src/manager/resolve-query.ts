/**
 * Query gateway path resolution.
 *
 * Resolves a parsed query gateway path to a concrete actor ID by calling the
 * appropriate manager driver method (getWithKey or getOrCreateWithKey).
 *
 * This is the TypeScript equivalent of the engine resolver at
 * `engine/packages/guard/src/routing/pegboard_gateway/resolve_actor_query.rs`.
 */
import type { Context as HonoContext } from "hono";
import * as errors from "@/actor/errors";
import type { RegistryConfig } from "@/registry/config";
import type {
	ParsedActorPath,
	ParsedDirectActorPath,
	ParsedQueryActorPath,
} from "./actor-path";
import type { ManagerDriver } from "./driver";
import { logger } from "./log";

/**
 * Resolve a parsed actor path to a direct actor path. If the path is already
 * direct, returns it unchanged. If it is a query path, resolves the query to
 * a concrete actor ID and returns a direct path.
 */
export async function resolvePathBasedActorPath(
	config: RegistryConfig,
	managerDriver: ManagerDriver,
	c: HonoContext,
	actorPathInfo: ParsedActorPath,
): Promise<ParsedDirectActorPath> {
	if (actorPathInfo.type === "direct") {
		return actorPathInfo;
	}

	assertQueryNamespaceMatchesConfig(config, actorPathInfo.namespace);

	const actorId = await resolveQueryActorId(
		managerDriver,
		c,
		actorPathInfo,
	);

	logger().debug({
		msg: "resolved query gateway path to actor",
		query: actorPathInfo.query,
		actorId,
	});

	return {
		type: "direct",
		actorId,
		token: actorPathInfo.token,
		remainingPath: actorPathInfo.remainingPath,
	};
}

/**
 * Resolve a query actor path to a concrete actor ID by dispatching to the
 * appropriate manager driver method.
 */
async function resolveQueryActorId(
	managerDriver: ManagerDriver,
	c: HonoContext,
	actorPathInfo: ParsedQueryActorPath,
): Promise<string> {
	const { query, crashPolicy } = actorPathInfo;

	if ("getForKey" in query) {
		const actorOutput = await managerDriver.getWithKey({
			c,
			name: query.getForKey.name,
			key: query.getForKey.key,
		});
		if (!actorOutput) {
			throw new errors.ActorNotFound(
				`${query.getForKey.name}:${JSON.stringify(query.getForKey.key)}`,
			);
		}
		return actorOutput.actorId;
	}

	if ("getOrCreateForKey" in query) {
		const actorOutput = await managerDriver.getOrCreateWithKey({
			c,
			name: query.getOrCreateForKey.name,
			key: query.getOrCreateForKey.key,
			input: query.getOrCreateForKey.input,
			region: query.getOrCreateForKey.region,
			crashPolicy,
		});
		return actorOutput.actorId;
	}

	const exhaustiveCheck: never = query;
	return exhaustiveCheck;
}

function assertQueryNamespaceMatchesConfig(
	config: RegistryConfig,
	namespace: string,
): void {
	if (namespace === config.namespace) {
		return;
	}

	throw new errors.InvalidRequest(
		`query gateway namespace '${namespace}' does not match manager namespace '${config.namespace}'`,
	);
}
