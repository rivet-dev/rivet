#!/usr/bin/env -S tsx

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

	result = postProcessAssert(result);

	await fs.writeFile(outputPath, result);
}

const ASSERT_FUNCTION = `
function assert(condition: boolean, message?: string): asserts condition {
    if (!condition) throw new Error(message ?? "Assertion failed")
}

`;

/**
 * Remove Node.js assert import and inject a custom assert function
 */
function postProcessAssert(code: string): string {
	// Remove Node.js assert import
	code = code.replace(/^import assert from "assert"/m, "");

	// Inject new assert function
	code += `\n${ASSERT_FUNCTION}`;

	return code;
}

export { type Config, transform } from "@bare-ts/tools";
