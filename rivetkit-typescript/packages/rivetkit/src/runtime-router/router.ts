import { createRoute } from "@hono/zod-openapi";
import * as cbor from "cbor-x";
import type { Hono } from "hono";
import invariant from "invariant";
import { z } from "zod/v4";
import { Forbidden, RestrictedFeature } from "@/actor/errors";
import { deserializeActorKey, serializeActorKey } from "@/actor/keys";
import {
	actorGateway,
	createTestWebSocketProxy,
} from "@/actor-gateway/gateway";
import type { Encoding } from "@/client/mod";
import {
	HEADER_RIVET_TOKEN,
	WS_PROTOCOL_ACTOR,
	WS_PROTOCOL_CONN_PARAMS,
	WS_PROTOCOL_ENCODING,
	WS_TEST_PROTOCOL_PATH,
} from "@/common/actor-router-consts";
import { handleHealthRequest, handleMetadataRequest } from "@/common/router";
import { deconstructError, noopNext, stringifyError } from "@/common/utils";

import { HEADER_ACTOR_ID } from "@/driver-helpers/mod";
import {
	ActorsCreateRequestSchema,
	type ActorsCreateResponse,
	ActorsCreateResponseSchema,
	ActorsGetOrCreateRequestSchema,
	type ActorsGetOrCreateResponse,
	ActorsGetOrCreateResponseSchema,
	type ActorsKvGetResponse,
	ActorsKvGetResponseSchema,
	type ActorsListNamesResponse,
	ActorsListNamesResponseSchema,
	type ActorsListResponse,
	ActorsListResponseSchema,
	type Actor as ApiActor,
} from "@/engine-api/actors";
import type { ActorOutput, EngineControlClient } from "@/engine-client/driver";
import { getInspectorDir } from "@/inspector/serve-ui";
import { buildActorNames, type RegistryConfig } from "@/registry/config";
import type { GetUpgradeWebSocket, Runtime } from "@/utils";
import { timingSafeEqual } from "@/utils/crypto";
import { isDev } from "@/utils/env-vars";
import {
	buildOpenApiRequestBody,
	buildOpenApiResponses,
	createRouter,
} from "@/utils/router";
import { loadRuntimeServeStatic } from "@/utils/serve";
import { logger } from "./log";

export function buildRuntimeRouter(
	config: RegistryConfig,
	engineClient: EngineControlClient,
	getUpgradeWebSocket: GetUpgradeWebSocket | undefined,
	runtime: Runtime = "node",
) {
	return createRouter(config.managerBasePath, (router) => {
		// Actor gateway
		router.use(
			"*",
			actorGateway.bind(
				undefined,
				config,
				engineClient,
				getUpgradeWebSocket,
			),
		);

		// GET /
		router.get("/", (c) => {
			return c.text(
				"This is a RivetKit server.\n\nLearn more at https://rivet.dev",
			);
		});

		// GET /actors
		{
			const route = createRoute({
				method: "get",
				path: "/actors",
				request: {
					query: z.object({
						name: z.string().optional(),
						actor_ids: z.string().optional(),
						key: z.string().optional(),
					}),
				},
				responses: buildOpenApiResponses(ActorsListResponseSchema),
			});

			router.openapi(route, async (c) => {
				const { name, actor_ids, key } = c.req.valid("query");

				const actorIdsParsed = actor_ids
					? actor_ids
						.split(",")
						.map((id) => id.trim())
						.filter((id) => id.length > 0)
					: undefined;

				const actors: ActorOutput[] = [];

				// Validate: cannot provide both actor_ids and (name or key)
				if (actorIdsParsed && (name || key)) {
					return c.json(
						{
							error: "Cannot provide both actor_ids and (name + key). Use either actor_ids or (name + key).",
						},
						400,
					);
				}

				// Validate: when key is provided, name must also be provided
				if (key && !name) {
					return c.json(
						{
							error: "Name is required when key is provided.",
						},
						400,
					);
				}

				if (actorIdsParsed) {
					if (actorIdsParsed.length > 32) {
						return c.json(
							{
								error: `Too many actor IDs. Maximum is 32, got ${actorIdsParsed.length}.`,
							},
							400,
						);
					}

					if (actorIdsParsed.length === 0) {
						return c.json<ActorsListResponse>({
							actors: [],
						});
					}

					// Fetch actors by ID
					for (const actorId of actorIdsParsed) {
						if (name) {
							// If name is provided, use it directly
							const actorOutput = await engineClient.getForId({
								c,
								name,
								actorId,
							});
							if (actorOutput) {
								actors.push(actorOutput);
							}
						} else {
							// If no name is provided, try all registered actor types
							// Actor IDs are globally unique, so we'll find it in one of them
							for (const actorName of Object.keys(config.use)) {
								const actorOutput = await engineClient.getForId(
									{
										c,
										name: actorName,
										actorId,
									},
								);
								if (actorOutput) {
									actors.push(actorOutput);
									break; // Found the actor, no need to check other names
								}
							}
						}
					}
				} else if (key && name) {
					const actorOutput = await engineClient.getWithKey({
						c,
						name,
						key: deserializeActorKey(key),
					});
					if (actorOutput) {
						actors.push(actorOutput);
					}
				} else {
					if (!name) {
						return c.json(
							{
								error: "Name is required when not using actor_ids.",
							},
							400,
						);
					}

					// List all actors with the given name
					const actorOutputs = await engineClient.listActors({
						c,
						name,
						key,
						includeDestroyed: false,
					});
					actors.push(...actorOutputs);
				}

				return c.json<ActorsListResponse>({
					actors: actors.map((actor) => createApiActor(actor)),
				});
			});
		}

		// GET /actors/names
		{
			const route = createRoute({
				method: "get",
				path: "/actors/names",
				request: {
					query: z.object({
						namespace: z.string(),
					}),
				},
				responses: buildOpenApiResponses(ActorsListNamesResponseSchema),
			});

			router.openapi(route, async (c) => {
				const names = buildActorNames(config);
				return c.json<ActorsListNamesResponse>({
					names,
				});
			});
		}

		// PUT /actors
		{
			const route = createRoute({
				method: "put",
				path: "/actors",
				request: {
					body: buildOpenApiRequestBody(
						ActorsGetOrCreateRequestSchema,
					),
				},
				responses: buildOpenApiResponses(
					ActorsGetOrCreateResponseSchema,
				),
			});

			router.openapi(route, async (c) => {
				const body = c.req.valid("json");

				// Check if actor already exists
				const existingActor = await engineClient.getWithKey({
					c,
					name: body.name,
					key: deserializeActorKey(body.key),
				});

				if (existingActor) {
					return c.json<ActorsGetOrCreateResponse>({
						actor: createApiActor(existingActor),
						created: false,
					});
				}

				// Create new actor
				const newActor = await engineClient.getOrCreateWithKey({
					c,
					name: body.name,
					key: deserializeActorKey(body.key),
					input: body.input
						? cbor.decode(Buffer.from(body.input, "base64"))
						: undefined,
					region: undefined, // Not provided in the request schema
				});

				return c.json<ActorsGetOrCreateResponse>({
					actor: createApiActor(newActor),
					created: true,
				});
			});
		}

		// POST /actors
		{
			const route = createRoute({
				method: "post",
				path: "/actors",
				request: {
					body: buildOpenApiRequestBody(ActorsCreateRequestSchema),
				},
				responses: buildOpenApiResponses(ActorsCreateResponseSchema),
			});

			router.openapi(route, async (c) => {
				const body = c.req.valid("json");

				// Create actor using the driver
				const actorOutput = await engineClient.createActor({
					c,
					name: body.name,
					key: deserializeActorKey(body.key || crypto.randomUUID()),
					input: body.input
						? cbor.decode(Buffer.from(body.input, "base64"))
						: undefined,
					region: undefined, // Not provided in the request schema
				});

				// Transform ActorOutput to match ActorSchema
				const actor = createApiActor(actorOutput);

				return c.json<ActorsCreateResponse>({ actor });
			});
		}

		// GET /actors/{actor_id}/kv/keys/{key}
		{
			const route = createRoute({
				method: "get",
				path: "/actors/{actor_id}/kv/keys/{key}",
				request: {
					params: z.object({
						actor_id: z.string(),
						key: z.string(),
					}),
				},
				responses: buildOpenApiResponses(ActorsKvGetResponseSchema),
			});

			router.openapi(route, async (c) => {
				if (isDev() && !config.token) {
					logger().warn({
						msg: "RIVET_TOKEN is not set, skipping KV store access checks in development mode. This endpoint will be disabled in production, unless you set the token.",
					});
				}

				if (!isDev()) {
					if (!config.token) {
						throw new RestrictedFeature("KV store access");
					}
					if (
						timingSafeEqual(
							config.token,
							c.req.header(HEADER_RIVET_TOKEN) || "",
						) === false
					) {
						throw new Forbidden();
					}
				}

				const { actor_id: actorId, key } = c.req.valid("param");

				const response = await engineClient.kvGet(
					actorId,
					Buffer.from(key, "base64"),
				);

				return c.json<ActorsKvGetResponse>({
					value: response
						? Buffer.from(response).toString("base64")
						: null,
				});
			});
		}

		// TODO:
		// // DELETE /actors/{actor_id}
		// {
		// 	const route = createRoute({
		// 		method: "delete",
		// 		path: "/actors/{actor_id}",
		// 		request: {
		// 			params: z.object({
		// 				actor_id: RivetIdSchema,
		// 			}),
		// 		},
		// 		responses: buildOpenApiResponses(
		// 			ActorsDeleteResponseSchema,
		// 			validateBody,
		// 		),
		// 	});
		//
		// 	router.openapi(route, async (c) => {
		// 		const { actor_id } = c.req.valid("param");
		//
		// 	});
		// }

		if (config.test.enabled) {
			// Test endpoint to force disconnect a connection non-cleanly
			router.post("/.test/force-disconnect", async (c) => {
				const actorId = c.req.query("actor");
				const connId = c.req.query("conn");

				if (!actorId || !connId) {
					return c.text(
						"Missing actor or conn query parameters",
						400,
					);
				}

				logger().debug({
					msg: "forcing unclean disconnect",
					actorId,
					connId,
				});

				try {
					// Send a special request to the actor to force disconnect the connection
					const response = await engineClient.sendRequest(
						{ directId: actorId },
						new Request(
							`http://actor/.test/force-disconnect?conn=${connId}`,
							{
								method: "POST",
							},
						),
					);

					if (!response.ok) {
						const text = await response.text();
						return c.text(
							`Failed to force disconnect: ${text}`,
							response.status as any,
						);
					}

					return c.json({ success: true });
				} catch (error) {
					logger().error({
						msg: "error forcing disconnect",
						error: stringifyError(error),
					});
					return c.text(`Error: ${error}`, 500);
				}
			});
		}

		if (config.inspector.enabled) {
			let inspectorRoot: string | undefined;

			router.get("/ui/*", async (c, next) => {
				let serveStatic;
				try {
					serveStatic = await loadRuntimeServeStatic(runtime);
				} catch (error) {
					logger().error({
						msg: "failed to load inspector static file handler",
						error: stringifyError(error),
					});
					return c.text(
						`Failed to load static file handler for runtime '${runtime}'.`,
						500,
					);
				}

				if (!inspectorRoot) {
					inspectorRoot = await getInspectorDir();
				}
				const root = inspectorRoot;
				const rewrite = (path: string) =>
					path.replace(/^\/ui/, "") || "/";

				return serveStatic({
					root,
					rewriteRequestPath: rewrite,
					onNotFound: async (_path, c) => {
						await serveStatic({ root, path: "index.html" })(
							c,
							next,
						);
					},
				})(c, next);
			});

			router.get("/ui", (c) => c.redirect("/ui/"));
		}

		router.get("/health", (c) => handleHealthRequest(c));

		router.get("/metadata", (c) =>
			handleMetadataRequest(
				c,
				config,
				{ normal: {} },
				config.publicEndpoint,
				config.publicNamespace,
				config.publicToken,
			),
		);

		engineClient.modifyRuntimeRouter?.(config, router as unknown as Hono);
	});
}

function createApiActor(actor: ActorOutput): ApiActor {
	return {
		actor_id: actor.actorId,
		name: actor.name,
		key: serializeActorKey(actor.key),
		namespace_id: "default", // Assert default namespace
		runner_name_selector: "default",
		create_ts: actor.createTs ?? Date.now(),
		connectable_ts: actor.connectableTs ?? null,
		destroy_ts: actor.destroyTs ?? null,
		sleep_ts: actor.sleepTs ?? null,
		start_ts: actor.startTs ?? null,
	};
}
