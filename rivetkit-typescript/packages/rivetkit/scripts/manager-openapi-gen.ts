import * as fs from "node:fs/promises";
import { resolve } from "node:path";
import { z } from "zod";
import { createFileSystemOrMemoryDriver } from "@/drivers/file-system/mod";
import type { ManagerDriver } from "@/manager/driver";
import { buildManagerRouter } from "@/manager/router";
import { type RegistryConfig, RegistryConfigSchema } from "@/registry/config";
import { VERSION } from "@/utils";
import { toJsonSchema } from "./schema-utils";

async function main() {
	const config: RegistryConfig = RegistryConfigSchema.parse({
		use: {},
		driver: createFileSystemOrMemoryDriver(false),
		getUpgradeWebSocket: () => () => unimplemented(),
		inspector: {
			enabled: false,
		},
	});
	// const registry = setup(registryConfig);

	const managerDriver: ManagerDriver = {
		getForId: unimplemented,
		getWithKey: unimplemented,
		getOrCreateWithKey: unimplemented,
		createActor: unimplemented,
		listActors: unimplemented,
		sendRequest: unimplemented,
		openWebSocket: unimplemented,
		proxyRequest: unimplemented,
		proxyWebSocket: unimplemented,
		displayInformation: unimplemented,
		setGetUpgradeWebSocket: unimplemented,
		buildGatewayUrl: unimplemented,
		kvGet: unimplemented,
	};

	// const client = createClientWithDriver(
	// 	managerDriver,
	// 	ClientConfigSchema.parse({}),
	// );
	//
	const { openapi: managerOpenapi } = buildManagerRouter(
		config,
		managerDriver,
		undefined,
	);

	// Get OpenAPI document
	const managerOpenApiDoc = managerOpenapi.getOpenAPIDocument({
		openapi: "3.0.0",
		info: {
			version: VERSION,
			title: "RivetKit API",
		},
	});

	// Inject actor router paths
	injectActorRouter(managerOpenApiDoc);

	const outputPath = resolve(
		import.meta.dirname,
		"..",
		"..",
		"..",
		"..",
		"rivetkit-openapi",
		"openapi.json",
	);
	await fs.writeFile(outputPath, JSON.stringify(managerOpenApiDoc, null, 2));
	console.log("Dumped OpenAPI to", outputPath);
}

// Schemas for action request/response
const HttpActionRequestSchema = z.object({
	args: z.unknown(),
});

const HttpActionResponseSchema = z.object({
	output: z.unknown(),
});

/**
 * Manually inject actor router paths into the OpenAPI spec.
 *
 * We do this manually instead of extracting from the actual router since the
 * actor routes support multiple encodings (JSON, CBOR, bare), but OpenAPI
 * specs are JSON-focused and don't cleanly represent multi-encoding routes.
 */
function injectActorRouter(openApiDoc: any) {
	if (!openApiDoc.paths) {
		openApiDoc.paths = {};
	}

	// Convert Zod schemas to JSON Schema
	const actionRequestSchema = toJsonSchema(HttpActionRequestSchema);
	delete (actionRequestSchema as any).$schema;

	const actionResponseSchema = toJsonSchema(HttpActionResponseSchema);
	delete (actionResponseSchema as any).$schema;

	// Common actorId parameter
	const actorIdParam = {
		name: "actorId",
		in: "path" as const,
		required: true,
		schema: {
			type: "string",
		},
		description: "The ID of the actor to target",
	};

	// GET /gateway/{actorId}/health
	openApiDoc.paths["/gateway/{actorId}/health"] = {
		get: {
			parameters: [actorIdParam],
			responses: {
				200: {
					description: "Health check",
					content: {
						"text/plain": {
							schema: {
								type: "string",
							},
						},
					},
				},
			},
		},
	};

	// POST /gateway/{actorId}/action/{action}
	openApiDoc.paths["/gateway/{actorId}/action/{action}"] = {
		post: {
			parameters: [
				actorIdParam,
				{
					name: "action",
					in: "path" as const,
					required: true,
					schema: {
						type: "string",
					},
					description: "The name of the action to execute",
				},
			],
			requestBody: {
				content: {
					"application/json": {
						schema: actionRequestSchema,
					},
				},
			},
			responses: {
				200: {
					description: "Action executed successfully",
					content: {
						"application/json": {
							schema: actionResponseSchema,
						},
					},
				},
				400: {
					description: "Invalid action",
				},
				500: {
					description: "Internal error",
				},
			},
		},
	};

	// ALL /gateway/{actorId}/request/{path}
	const requestPath = {
		parameters: [
			actorIdParam,
			{
				name: "path",
				in: "path" as const,
				required: true,
				schema: {
					type: "string",
				},
				description: "The HTTP path to forward to the actor",
			},
		],
		responses: {
			200: {
				description: "Response from actor's raw HTTP handler",
			},
		},
	};

	openApiDoc.paths["/gateway/{actorId}/request/{path}"] = {
		get: requestPath,
		post: requestPath,
		put: requestPath,
		delete: requestPath,
		patch: requestPath,
		head: requestPath,
		options: requestPath,
	};

	// Inspector endpoints
	const inspectorAuthHeader = {
		name: "Authorization",
		in: "header" as const,
		required: false,
		schema: {
			type: "string",
		},
		description:
			"Bearer token for inspector authentication. Required in production, optional in development.",
	};

	// GET /gateway/{actorId}/inspector/state
	openApiDoc.paths["/gateway/{actorId}/inspector/state"] = {
		get: {
			parameters: [actorIdParam, inspectorAuthHeader],
			responses: {
				200: {
					description: "Current actor state",
					content: {
						"application/json": {
							schema: {
								type: "object",
								properties: {
									state: {},
								},
							},
						},
					},
				},
				401: { description: "Unauthorized" },
			},
		},
		patch: {
			parameters: [actorIdParam, inspectorAuthHeader],
			requestBody: {
				content: {
					"application/json": {
						schema: {
							type: "object",
							properties: {
								state: {},
							},
							required: ["state"],
						},
					},
				},
			},
			responses: {
				200: {
					description: "State updated",
					content: {
						"application/json": {
							schema: {
								type: "object",
								properties: {
									ok: { type: "boolean" },
								},
							},
						},
					},
				},
				401: { description: "Unauthorized" },
			},
		},
	};

	// GET /gateway/{actorId}/inspector/connections
	openApiDoc.paths["/gateway/{actorId}/inspector/connections"] = {
		get: {
			parameters: [actorIdParam, inspectorAuthHeader],
			responses: {
				200: {
					description: "Current actor connections",
					content: {
						"application/json": {
							schema: {
								type: "object",
								properties: {
									connections: {
										type: "array",
										items: { type: "object" },
									},
								},
							},
						},
					},
				},
				401: { description: "Unauthorized" },
			},
		},
	};

	// GET /gateway/{actorId}/inspector/rpcs
	openApiDoc.paths["/gateway/{actorId}/inspector/rpcs"] = {
		get: {
			parameters: [actorIdParam, inspectorAuthHeader],
			responses: {
				200: {
					description: "Available actor actions/RPCs",
					content: {
						"application/json": {
							schema: {
								type: "object",
								properties: {
									rpcs: { type: "object" },
								},
							},
						},
					},
				},
				401: { description: "Unauthorized" },
			},
		},
	};

	// POST /gateway/{actorId}/inspector/action/{name}
	openApiDoc.paths["/gateway/{actorId}/inspector/action/{name}"] = {
		post: {
			parameters: [
				actorIdParam,
				{
					name: "name",
					in: "path" as const,
					required: true,
					schema: {
						type: "string",
					},
					description: "The name of the action to execute",
				},
				inspectorAuthHeader,
			],
			requestBody: {
				content: {
					"application/json": {
						schema: {
							type: "object",
							properties: {
								args: {
									type: "array",
									items: {},
								},
							},
						},
					},
				},
			},
			responses: {
				200: {
					description: "Action executed successfully",
					content: {
						"application/json": {
							schema: {
								type: "object",
								properties: {
									output: {},
								},
							},
						},
					},
				},
				401: { description: "Unauthorized" },
			},
		},
	};

	// GET /gateway/{actorId}/inspector/queue
	openApiDoc.paths["/gateway/{actorId}/inspector/queue"] = {
		get: {
			parameters: [
				actorIdParam,
				{
					name: "limit",
					in: "query" as const,
					required: false,
					schema: {
						type: "integer",
						default: 50,
					},
					description: "Maximum number of queue messages to return",
				},
				inspectorAuthHeader,
			],
			responses: {
				200: {
					description: "Queue status",
					content: {
						"application/json": {
							schema: {
								type: "object",
								properties: {
									size: { type: "integer" },
									maxSize: { type: "integer" },
									truncated: { type: "boolean" },
									messages: {
										type: "array",
										items: {
											type: "object",
											properties: {
												id: { type: "string" },
												name: { type: "string" },
												createdAtMs: { type: "integer" },
											},
										},
									},
								},
							},
						},
					},
				},
				401: { description: "Unauthorized" },
			},
		},
	};

	// GET /gateway/{actorId}/inspector/traces
	openApiDoc.paths["/gateway/{actorId}/inspector/traces"] = {
		get: {
			parameters: [
				actorIdParam,
				{
					name: "startMs",
					in: "query" as const,
					required: false,
					schema: {
						type: "integer",
						default: 0,
					},
					description: "Start of time range in epoch milliseconds",
				},
				{
					name: "endMs",
					in: "query" as const,
					required: false,
					schema: {
						type: "integer",
					},
					description:
						"End of time range in epoch milliseconds. Defaults to now.",
				},
				{
					name: "limit",
					in: "query" as const,
					required: false,
					schema: {
						type: "integer",
						default: 1000,
					},
					description: "Maximum number of spans to return",
				},
				inspectorAuthHeader,
			],
			responses: {
				200: {
					description: "Trace spans in OTLP JSON format",
					content: {
						"application/json": {
							schema: {
								type: "object",
								properties: {
									otlp: { type: "object" },
									clamped: { type: "boolean" },
								},
							},
						},
					},
				},
				401: { description: "Unauthorized" },
			},
		},
	};

	// GET /gateway/{actorId}/inspector/workflow-history
	openApiDoc.paths["/gateway/{actorId}/inspector/workflow-history"] = {
		get: {
			parameters: [actorIdParam, inspectorAuthHeader],
			responses: {
				200: {
					description: "Workflow history and status",
					content: {
						"application/json": {
							schema: {
								type: "object",
								properties: {
									history: {},
									isWorkflowEnabled: { type: "boolean" },
								},
							},
						},
					},
				},
				401: { description: "Unauthorized" },
			},
		},
	};

	// GET /gateway/{actorId}/inspector/summary
	openApiDoc.paths["/gateway/{actorId}/inspector/summary"] = {
		get: {
			parameters: [actorIdParam, inspectorAuthHeader],
			responses: {
				200: {
					description: "Full actor inspector summary",
					content: {
						"application/json": {
							schema: {
								type: "object",
								properties: {
									state: {},
									connections: {
										type: "array",
										items: { type: "object" },
									},
									rpcs: { type: "object" },
									queueSize: { type: "integer" },
									isStateEnabled: { type: "boolean" },
									isDatabaseEnabled: { type: "boolean" },
									isWorkflowEnabled: { type: "boolean" },
									workflowHistory: {},
								},
							},
						},
					},
				},
				401: { description: "Unauthorized" },
			},
		},
	};
}

function unimplemented(): never {
	throw new Error("UNIMPLEMENTED");
}

// biome-ignore lint/nursery/noFloatingPromises: main
main();
