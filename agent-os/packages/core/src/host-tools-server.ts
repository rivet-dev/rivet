import {
	createServer,
	type IncomingMessage,
	type Server,
	type ServerResponse,
} from "node:http";
import type { HostTool, ToolKit } from "./host-tools.js";
import {
	camelToKebab,
	getFieldInfos,
	getZodDescription,
	getZodEnumValues,
	parseArgv,
} from "./host-tools-argv.js";

const DEFAULT_TIMEOUT = 30000;

interface CallRequest {
	toolkit: string;
	tool: string;
	input?: unknown;
	argv?: string[];
}

interface RpcSuccess {
	ok: true;
	result: unknown;
}

interface RpcError {
	ok: false;
	error: string;
	message: string;
}

type RpcResponse = RpcSuccess | RpcError;

function errorResponse(error: string, message: string): RpcError {
	return { ok: false, error, message };
}

function readBody(req: IncomingMessage): Promise<string> {
	return new Promise((resolve, reject) => {
		const chunks: Buffer[] = [];
		req.on("data", (chunk: Buffer) => chunks.push(chunk));
		req.on("end", () => resolve(Buffer.concat(chunks).toString("utf-8")));
		req.on("error", reject);
	});
}

function sendJson(res: ServerResponse, body: RpcResponse): void {
	res.writeHead(200, { "Content-Type": "application/json" });
	res.end(JSON.stringify(body));
}

function toolkitNames(toolkits: Map<string, ToolKit>): string {
	return [...toolkits.keys()].join(", ");
}

function toolNames(toolkit: ToolKit): string {
	return Object.keys(toolkit.tools).join(", ");
}

async function handleCall(
	body: string,
	toolkits: Map<string, ToolKit>,
): Promise<RpcResponse> {
	let parsed: CallRequest;
	try {
		parsed = JSON.parse(body) as CallRequest;
	} catch {
		return errorResponse(
			"VALIDATION_ERROR",
			"Invalid JSON in request body",
		);
	}

	const { toolkit: tkName, tool: toolName, input, argv } = parsed;

	// Look up toolkit
	const tk = toolkits.get(tkName);
	if (!tk) {
		return errorResponse(
			"TOOLKIT_NOT_FOUND",
			`No toolkit "${tkName}". Available: ${toolkitNames(toolkits)}`,
		);
	}

	// Look up tool
	const tool = tk.tools[toolName];
	if (!tool) {
		return errorResponse(
			"TOOL_NOT_FOUND",
			`No tool "${toolName}" in toolkit "${tkName}". Available: ${toolNames(tk)}`,
		);
	}

	// If argv is provided, parse flags against the zod schema to produce input
	let resolvedInput: unknown = input ?? {};
	if (argv) {
		const argvResult = parseArgv(tool.inputSchema, argv);
		if (!argvResult.ok) {
			return errorResponse("VALIDATION_ERROR", argvResult.message);
		}
		resolvedInput = argvResult.input;
	}

	// Validate input against zod schema
	const parseResult = tool.inputSchema.safeParse(resolvedInput);
	if (!parseResult.success) {
		const message = parseResult.error.errors
			.map((e) => {
				const path =
					e.path.length > 0 ? `at "${e.path.join(".")}"` : "";
				return `${e.message}${path ? ` ${path}` : ""}`;
			})
			.join("; ");
		return errorResponse("VALIDATION_ERROR", message);
	}

	// Execute with timeout
	const timeout = tool.timeout ?? DEFAULT_TIMEOUT;
	try {
		const result = await Promise.race([
			Promise.resolve(tool.execute(parseResult.data)),
			new Promise<never>((_, reject) =>
				setTimeout(() => reject(new Error(`TIMEOUT`)), timeout),
			),
		]);
		return { ok: true, result };
	} catch (err: unknown) {
		const message = err instanceof Error ? err.message : String(err);
		if (message === "TIMEOUT") {
			return errorResponse(
				"TIMEOUT",
				`Tool "${toolName}" timed out after ${timeout}ms`,
			);
		}
		return errorResponse("EXECUTION_ERROR", message);
	}
}

interface FlagDescriptor {
	flag: string;
	type: string;
	required: boolean;
	description?: string;
}

interface ToolDescriptor {
	description: string;
	flags: FlagDescriptor[];
	examples?: Array<{ description: string; input: unknown }>;
}

/**
 * Extract flag descriptors from a HostTool's zod input schema.
 */
function describeFlags(tool: HostTool): FlagDescriptor[] {
	const fields = getFieldInfos(tool.inputSchema);
	const flags: FlagDescriptor[] = [];
	const shape: Record<string, unknown> =
		(tool.inputSchema as any)._def.typeName === "ZodObject"
			? (tool.inputSchema as any)._def.shape()
			: {};

	for (const field of fields.values()) {
		let type: string;
		if (field.innerTypeName === "ZodString") {
			type = "string";
		} else if (field.innerTypeName === "ZodNumber") {
			type = "number";
		} else if (field.innerTypeName === "ZodBoolean") {
			type = "boolean";
		} else if (field.innerTypeName === "ZodEnum") {
			const fieldSchema = shape[field.camelName];
			const values = fieldSchema
				? getZodEnumValues(fieldSchema as any)
				: undefined;
			type = values ? values.join("|") : "enum";
		} else if (field.innerTypeName === "ZodArray") {
			const itemType =
				field.arrayItemTypeName === "ZodNumber" ? "number" : "string";
			type = `${itemType}[]`;
		} else {
			type = "string";
		}

		const fieldSchema = shape[field.camelName];
		const description = fieldSchema
			? getZodDescription(fieldSchema as any)
			: undefined;

		const descriptor: FlagDescriptor = {
			flag: `--${camelToKebab(field.camelName)}`,
			type,
			required: !field.isOptional,
		};
		if (description) {
			descriptor.description = description;
		}
		flags.push(descriptor);
	}

	return flags;
}

/**
 * Build a full tool descriptor with flags and examples.
 */
function describeTool(tool: HostTool): ToolDescriptor {
	const descriptor: ToolDescriptor = {
		description: tool.description,
		flags: describeFlags(tool),
	};
	if (tool.examples && tool.examples.length > 0) {
		descriptor.examples = tool.examples.map((ex) => ({
			description: ex.description,
			input: ex.input,
		}));
	}
	return descriptor;
}

function handleList(toolkits: Map<string, ToolKit>): RpcResponse {
	const result = [];
	for (const tk of toolkits.values()) {
		result.push({
			name: tk.name,
			description: tk.description,
			tools: Object.keys(tk.tools),
		});
	}
	return { ok: true, result: { toolkits: result } };
}

function handleListToolkit(
	toolkitName: string,
	toolkits: Map<string, ToolKit>,
): RpcResponse {
	const tk = toolkits.get(toolkitName);
	if (!tk) {
		return errorResponse(
			"TOOLKIT_NOT_FOUND",
			`No toolkit "${toolkitName}". Available: ${toolkitNames(toolkits)}`,
		);
	}

	const tools: Record<
		string,
		{ description: string; flags: FlagDescriptor[] }
	> = {};
	for (const [name, tool] of Object.entries(tk.tools)) {
		tools[name] = {
			description: tool.description,
			flags: describeFlags(tool),
		};
	}

	return {
		ok: true,
		result: { name: tk.name, description: tk.description, tools },
	};
}

function handleDescribeToolkit(
	toolkitName: string,
	toolkits: Map<string, ToolKit>,
): RpcResponse {
	const tk = toolkits.get(toolkitName);
	if (!tk) {
		return errorResponse(
			"TOOLKIT_NOT_FOUND",
			`No toolkit "${toolkitName}". Available: ${toolkitNames(toolkits)}`,
		);
	}

	const tools: Record<string, ToolDescriptor> = {};
	for (const [name, tool] of Object.entries(tk.tools)) {
		tools[name] = describeTool(tool);
	}

	return {
		ok: true,
		result: { name: tk.name, description: tk.description, tools },
	};
}

function handleDescribeTool(
	toolkitName: string,
	toolName: string,
	toolkits: Map<string, ToolKit>,
): RpcResponse {
	const tk = toolkits.get(toolkitName);
	if (!tk) {
		return errorResponse(
			"TOOLKIT_NOT_FOUND",
			`No toolkit "${toolkitName}". Available: ${toolkitNames(toolkits)}`,
		);
	}

	const tool = tk.tools[toolName];
	if (!tool) {
		return errorResponse(
			"TOOL_NOT_FOUND",
			`No tool "${toolName}" in toolkit "${toolkitName}". Available: ${toolNames(tk)}`,
		);
	}

	return {
		ok: true,
		result: {
			toolkit: toolkitName,
			tool: toolName,
			...describeTool(tool),
		},
	};
}

export interface HostToolsServer {
	/** The port the server is listening on. */
	port: number;
	/** Register additional toolkits. */
	registerToolkit(toolkit: ToolKit): void;
	/** Shut down the HTTP server. */
	close(): Promise<void>;
}

/**
 * Start the host tools RPC server on 127.0.0.1:0.
 * Returns a handle with the assigned port.
 */
export function startHostToolsServer(
	toolkits: ToolKit[],
): Promise<HostToolsServer> {
	const toolkitMap = new Map<string, ToolKit>();
	for (const tk of toolkits) {
		toolkitMap.set(tk.name, tk);
	}

	return new Promise((resolve, reject) => {
		const server: Server = createServer(
			async (req: IncomingMessage, res: ServerResponse) => {
				const url = req.url ?? "/";
				const method = req.method ?? "GET";

				if (method === "POST" && url === "/call") {
					const body = await readBody(req);
					const result = await handleCall(body, toolkitMap);
					sendJson(res, result);
					return;
				}

				if (method === "GET" && url === "/list") {
					sendJson(res, handleList(toolkitMap));
					return;
				}

				// GET /list/<toolkit>
				if (method === "GET" && url.startsWith("/list/")) {
					const tkName = decodeURIComponent(
						url.slice("/list/".length),
					);
					sendJson(res, handleListToolkit(tkName, toolkitMap));
					return;
				}

				// GET /describe/<toolkit>/<tool> (must match before /describe/<toolkit>)
				if (method === "GET" && url.startsWith("/describe/")) {
					const rest = url.slice("/describe/".length);
					const slashIdx = rest.indexOf("/");
					if (slashIdx !== -1) {
						const tkName = decodeURIComponent(
							rest.slice(0, slashIdx),
						);
						const toolName = decodeURIComponent(
							rest.slice(slashIdx + 1),
						);
						sendJson(
							res,
							handleDescribeTool(tkName, toolName, toolkitMap),
						);
						return;
					}
					// GET /describe/<toolkit>
					const tkName = decodeURIComponent(rest);
					sendJson(res, handleDescribeToolkit(tkName, toolkitMap));
					return;
				}

				// Unknown route
				sendJson(
					res,
					errorResponse(
						"INTERNAL_ERROR",
						`Unknown endpoint: ${method} ${url}`,
					),
				);
			},
		);

		server.listen(0, "127.0.0.1", () => {
			const addr = server.address();
			if (!addr || typeof addr === "string") {
				reject(new Error("Failed to get server address"));
				return;
			}
			resolve({
				port: addr.port,
				registerToolkit(toolkit: ToolKit) {
					toolkitMap.set(toolkit.name, toolkit);
				},
				close() {
					return new Promise<void>((res, rej) => {
						server.close((err) => {
							if (err) rej(err);
							else res();
						});
					});
				},
			});
		});

		server.on("error", reject);
	});
}
