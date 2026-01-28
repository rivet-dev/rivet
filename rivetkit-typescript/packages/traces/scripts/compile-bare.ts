#!/usr/bin/env -S tsx

/**
 * BARE schema compiler for TypeScript
 *
 * This script compiles .bare schema files to TypeScript using @bare-ts/tools,
 * then post-processes the output to:
 * 1. Replace @bare-ts/lib import with @rivetkit/bare-ts
 * 2. Replace Node.js assert import with a custom assert function
 */

import * as fs from "node:fs/promises";
import * as path from "node:path";
import { type Config, transform } from "@bare-ts/tools";
import { Command } from "commander";

const program = new Command();

program
	.name("bare-compiler")
	.description("Compile BARE schemas to TypeScript")
	.version("0.0.1");

program
	.command("compile")
	.description("Compile a BARE schema file")
	.argument("<input>", "Input BARE schema file")
	.option("-o, --output <file>", "Output file path")
	.option("--pedantic", "Enable pedantic mode", false)
	.option("--generator <type>", "Generator type (ts, js, dts, bare)", "ts")
	.action(async (input: string, options) => {
		try {
			const schemaPath = path.resolve(input);
			const outputPath = options.output
				? path.resolve(options.output)
				: schemaPath.replace(/\.bare$/, ".ts");

			await compileSchema({
				schemaPath,
				outputPath,
				config: {
					pedantic: options.pedantic,
					generator: options.generator,
				},
			});

			console.log(`Successfully compiled ${input} to ${outputPath}`);
		} catch (error) {
			console.error("Failed to compile schema:", error);
			process.exit(1);
		}
	});

program.parse();

export interface CompileOptions {
	schemaPath: string;
	outputPath: string;
	config?: Partial<Config>;
}

export async function compileSchema(options: CompileOptions): Promise<void> {
	const { schemaPath, outputPath, config = {} } = options;

	const schema = await fs.readFile(schemaPath, "utf-8");
	const outputDir = path.dirname(outputPath);

	await fs.mkdir(outputDir, { recursive: true });

	const defaultConfig: Partial<Config> = {
		pedantic: true,
		generator: "ts",
		...config,
	};

	let result = transform(schema, defaultConfig);

	result = postProcess(result);

	await fs.writeFile(outputPath, result);
}

const POST_PROCESS_MARKER = "// @generated - post-processed by compile-bare.ts\n";

const ASSERT_FUNCTION = `
function assert(condition: boolean, message?: string): asserts condition {
    if (!condition) throw new Error(message ?? "Assertion failed")
}
`;

/**
 * Post-process the generated TypeScript file to:
 * 1. Replace @bare-ts/lib import with @rivetkit/bare-ts
 * 2. Replace Node.js assert import with a custom assert function
 */
function postProcess(code: string): string {
	if (code.startsWith(POST_PROCESS_MARKER)) {
		return code;
	}

	code = code.replace(/@bare-ts\/lib/g, "@rivetkit/bare-ts");
	code = code.replace(/^import assert from "assert"/m, "");
	code = POST_PROCESS_MARKER + code + `\n${ASSERT_FUNCTION}`;

	if (code.includes("@bare-ts/lib")) {
		throw new Error("Failed to replace @bare-ts/lib import");
	}
	if (code.includes("import assert from")) {
		throw new Error("Failed to remove Node.js assert import");
	}

	return code;
}

export type { Config } from "@bare-ts/tools";
