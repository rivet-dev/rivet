import * as fs from "node:fs/promises";
import { resolve } from "node:path";
import { z } from "zod";
import { zodToJsonSchema } from "zod-to-json-schema";
import { ClientConfigSchema } from "@/client/config";
import { createFileSystemOrMemoryDriver } from "@/drivers/file-system/mod";
import type {
	ActorOutput,
	ListActorsInput,
	ManagerDriver,
} from "@/manager/driver";
import { createManagerRouter } from "@/manager/router";
import {
	createClientWithDriver,
	type RegistryConfig,
	RegistryConfigSchema,
	setup,
} from "@/mod";
import { type RunnerConfig, RunnerConfigSchema } from "@/registry/run-config";
import { VERSION } from "@/utils";

async function main() {
	const registryConfig: RegistryConfig = RegistryConfigSchema.parse({
		use: {},
	});
	const registry = setup(registryConfig);

	const driverConfig: RunnerConfig = RunnerConfigSchema.parse({
		driver: createFileSystemOrMemoryDriver(false),
		getUpgradeWebSocket: () => () => unimplemented(),
		inspector: {
			enabled: false,
		},
	});

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
	};

	const client = createClientWithDriver(
		managerDriver,
		ClientConfigSchema.parse({}),
	);

	const { openapi: managerOpenapi } = createManagerRouter(
		registryConfig,
		driverConfig,
		managerDriver,
		driverConfig.driver!,
		client,
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

	// Convert Zod schemas to JSON Schema and remove $schema property
	const actionRequestSchema = zodToJsonSchema(HttpActionRequestSchema, {
		$refStrategy: "none",
	});
	delete (actionRequestSchema as any).$schema;

	const actionResponseSchema = zodToJsonSchema(HttpActionResponseSchema, {
		$refStrategy: "none",
	});
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
}

function unimplemented(): never {
	throw new Error("UNIMPLEMENTED");
}

main();
