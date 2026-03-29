import type { ToolKit } from "./host-tools.js";
import {
	camelToKebab,
	getFieldInfos,
	getZodDescription,
	getZodEnumValues,
} from "./host-tools-argv.js";

/**
 * Generate a markdown tool reference from a list of toolkits.
 * One line per tool in the summary to keep prompt size manageable.
 * Agents can run `--help` for full details.
 */
export function generateToolReference(toolKits: ToolKit[]): string {
	if (toolKits.length === 0) return "";

	const lines: string[] = [];
	lines.push("## Available Host Tools");
	lines.push("");
	lines.push(
		"Run `agentos list-tools` to see all available tools.",
	);
	lines.push("");

	for (const tk of toolKits) {
		lines.push(`### ${tk.name}`);
		lines.push("");
		lines.push(tk.description);
		lines.push("");

		for (const [toolName, tool] of Object.entries(tk.tools)) {
			// Build flag signature
			const flagSig = buildFlagSignature(tool.inputSchema);
			const flagStr = flagSig ? ` ${flagSig}` : "";
			lines.push(
				`- \`agentos-${tk.name} ${toolName}${flagStr}\` — ${tool.description}`,
			);
		}
		lines.push("");

		// Include examples if any tool has them
		const toolsWithExamples = Object.entries(tk.tools).filter(
			([, t]) => t.examples && t.examples.length > 0,
		);
		if (toolsWithExamples.length > 0) {
			lines.push("**Examples:**");
			lines.push("");
			for (const [toolName, tool] of toolsWithExamples) {
				for (const ex of tool.examples ?? []) {
					const flagArgs = inputToFlags(ex.input);
					lines.push(
						`- ${ex.description}: \`agentos-${tk.name} ${toolName}${flagArgs ? ` ${flagArgs}` : ""}\``,
					);
				}
			}
			lines.push("");
		}

		lines.push(
			`Run \`agentos-${tk.name} <tool> --help\` for details.`,
		);
		lines.push("");
	}

	return lines.join("\n");
}

/**
 * Build a compact flag signature from a Zod schema.
 * Example: `--a <number> --b <number>`
 */
function buildFlagSignature(schema: any): string {
	const fields = getFieldInfos(schema);
	const shape: Record<string, unknown> =
		schema._def?.typeName === "ZodObject" ? schema._def.shape() : {};

	const parts: string[] = [];
	for (const field of fields.values()) {
		const flag = `--${camelToKebab(field.camelName)}`;

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

		if (field.isOptional) {
			parts.push(`[${flag} <${type}>]`);
		} else {
			parts.push(`${flag} <${type}>`);
		}
	}

	return parts.join(" ");
}

/**
 * Convert an example input object to CLI flag string.
 */
function inputToFlags(input: unknown): string {
	if (!input || typeof input !== "object") return "";

	const parts: string[] = [];
	for (const [key, value] of Object.entries(input as Record<string, unknown>)) {
		const flag = `--${camelToKebab(key)}`;
		if (typeof value === "boolean") {
			if (value) parts.push(flag);
		} else if (Array.isArray(value)) {
			for (const item of value) {
				parts.push(`${flag} ${String(item)}`);
			}
		} else {
			parts.push(`${flag} ${String(value)}`);
		}
	}
	return parts.join(" ");
}
