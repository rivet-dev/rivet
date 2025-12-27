import * as fs from "node:fs/promises";
import { resolve } from "node:path";
import {
	ActionRequestSchema,
	ActionResponseSchema,
	ErrorSchema,
	EventSchema,
	InitSchema,
	SubscriptionRequestSchema,
	ToClientSchema,
	ToServerSchema,
} from "@/schemas/client-protocol-zod/mod";
import { VERSION } from "@/utils";
import { toJsonSchema } from "./schema-utils";

// Helper function to fix $ref paths from #/definitions to #/components/schemas
function fixRefs(obj: any): any {
	if (Array.isArray(obj)) {
		return obj.map(fixRefs);
	}
	if (obj && typeof obj === "object") {
		const newObj: any = {};
		for (const [key, value] of Object.entries(obj)) {
			if (
				key === "$ref" &&
				typeof value === "string" &&
				value.startsWith("#/definitions/")
			) {
				newObj[key] = value.replace(
					"#/definitions/",
					"#/components/schemas/",
				);
			} else if (key === "definitions") {
			} else {
				newObj[key] = fixRefs(value);
			}
		}
		return newObj;
	}
	return obj;
}

// Helper function to extract and flatten definitions into schemas
function extractSchemas(jsonSchema: any): Record<string, any> {
	const schemas: Record<string, any> = {};

	if (jsonSchema.definitions) {
		for (const [name, schema] of Object.entries(jsonSchema.definitions)) {
			schemas[name] = fixRefs(schema);
		}
	}

	return schemas;
}

// Helper function to get schema without definitions wrapper
function getSchemaWithoutDefinitions(jsonSchema: any): any {
	const result = { ...jsonSchema };
	delete result.$schema;
	delete result.definitions;

	// If there's a $ref to definitions, replace it with the actual schema
	if (result.$ref && result.$ref.startsWith("#/definitions/")) {
		const defName = result.$ref.replace("#/definitions/", "");
		if (jsonSchema.definitions && jsonSchema.definitions[defName]) {
			return fixRefs(jsonSchema.definitions[defName]);
		}
	}

	return fixRefs(result);
}

function main() {
	// Convert Zod schemas to JSON schemas using native z.toJSONSchema with BigInt support
	const toClientJsonSchema = toJsonSchema(ToClientSchema);
	const toServerJsonSchema = toJsonSchema(ToServerSchema);
	const initJsonSchema = toJsonSchema(InitSchema);
	const errorJsonSchema = toJsonSchema(ErrorSchema);
	const actionResponseJsonSchema = toJsonSchema(ActionResponseSchema);
	const eventJsonSchema = toJsonSchema(EventSchema);
	const actionRequestJsonSchema = toJsonSchema(ActionRequestSchema);
	const subscriptionRequestJsonSchema = toJsonSchema(
		SubscriptionRequestSchema,
	);

	// Build AsyncAPI v3.0.0 specification
	const asyncApiSpec = {
		asyncapi: "3.0.0",
		info: {
			title: "RivetKit WebSocket Protocol",
			version: VERSION,
			description:
				"WebSocket protocol for bidirectional communication between RivetKit clients and actors",
		},
		channels: {
			"/gateway/{actorId}/connect": {
				address: "/gateway/{actorId}/connect",
				parameters: {
					actorId: {
						description:
							"The unique identifier for the actor instance",
					},
				},
				messages: {
					toClient: {
						$ref: "#/components/messages/ToClient",
					},
					toServer: {
						$ref: "#/components/messages/ToServer",
					},
				},
			},
		},
		operations: {
			sendToClient: {
				action: "send",
				channel: {
					$ref: "#/channels/~1gateway~1{actorId}~1connect",
				},
				messages: [
					{
						$ref: "#/channels/~1gateway~1{actorId}~1connect/messages/toClient",
					},
				],
				summary: "Send messages from server to client",
				description:
					"Messages sent from the RivetKit actor to connected clients",
			},
			receiveFromClient: {
				action: "receive",
				channel: {
					$ref: "#/channels/~1gateway~1{actorId}~1connect",
				},
				messages: [
					{
						$ref: "#/channels/~1gateway~1{actorId}~1connect/messages/toServer",
					},
				],
				summary: "Receive messages from client",
				description:
					"Messages received by the RivetKit actor from connected clients",
			},
		},
		components: {
			messages: {
				ToClient: {
					name: "ToClient",
					title: "Message To Client",
					summary: "A message sent from the server to the client",
					contentType: "application/json",
					payload: getSchemaWithoutDefinitions(toClientJsonSchema),
					examples: [
						{
							name: "Init message",
							summary: "Initial connection message",
							payload: {
								body: {
									tag: "Init",
									val: {
										actorId: "actor_123",
										connectionId: "conn_456",
									},
								},
							},
						},
						{
							name: "Error message",
							summary: "Error response",
							payload: {
								body: {
									tag: "Error",
									val: {
										group: "auth",
										code: "unauthorized",
										message: "Authentication failed",
										actionId: null,
									},
								},
							},
						},
						{
							name: "Action response",
							summary: "Response to an action request",
							payload: {
								body: {
									tag: "ActionResponse",
									val: {
										id: "123",
										output: { result: "success" },
									},
								},
							},
						},
						{
							name: "Event",
							summary: "Event broadcast to subscribed clients",
							payload: {
								body: {
									tag: "Event",
									val: {
										name: "stateChanged",
										args: { newState: "active" },
									},
								},
							},
						},
					],
				},
				ToServer: {
					name: "ToServer",
					title: "Message To Server",
					summary: "A message sent from the client to the server",
					contentType: "application/json",
					payload: getSchemaWithoutDefinitions(toServerJsonSchema),
					examples: [
						{
							name: "Action request",
							summary: "Request to execute an action",
							payload: {
								body: {
									tag: "ActionRequest",
									val: {
										id: "123",
										name: "updateState",
										args: { key: "value" },
									},
								},
							},
						},
						{
							name: "Subscription request",
							summary:
								"Request to subscribe/unsubscribe from an event",
							payload: {
								body: {
									tag: "SubscriptionRequest",
									val: {
										eventName: "stateChanged",
										subscribe: true,
									},
								},
							},
						},
					],
				},
			},
			schemas: {
				Init: {
					...getSchemaWithoutDefinitions(initJsonSchema),
					description:
						"Initial connection message sent from server to client",
				},
				Error: {
					...getSchemaWithoutDefinitions(errorJsonSchema),
					description: "Error message sent from server to client",
				},
				ActionResponse: {
					...getSchemaWithoutDefinitions(actionResponseJsonSchema),
					description: "Response to an action request",
				},
				Event: {
					...getSchemaWithoutDefinitions(eventJsonSchema),
					description: "Event broadcast to subscribed clients",
				},
				ActionRequest: {
					...getSchemaWithoutDefinitions(actionRequestJsonSchema),
					description: "Request to execute an action on the actor",
				},
				SubscriptionRequest: {
					...getSchemaWithoutDefinitions(
						subscriptionRequestJsonSchema,
					),
					description:
						"Request to subscribe or unsubscribe from an event",
				},
			},
		},
	};

	const outputPath = resolve(
		import.meta.dirname,
		"..",
		"..",
		"..",
		"..",
		"rivetkit-asyncapi",
		"asyncapi.json",
	);

	fs.writeFile(outputPath, JSON.stringify(asyncApiSpec, null, 2));
	console.log("Dumped AsyncAPI spec to", outputPath);
}

main();
