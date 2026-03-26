#!/usr/bin/env -S tsx

/**
 * Compiles all .bare schema files under schemas/ to TypeScript.
 *
 * Each schemas/<group>/<version>.bare is compiled to
 * dist/schemas/<group>/<version>.ts. Adding a new schema file requires no
 * changes to package.json.
 */

import * as fs from "node:fs/promises";
import * as path from "node:path";
import { compileSchema } from "./compile-bare.js";

const schemasDir = path.resolve(import.meta.dirname, "../schemas");
const distDir = path.resolve(import.meta.dirname, "../dist/schemas");

async function findBareFiles(dir: string): Promise<string[]> {
	const entries = await fs.readdir(dir, { withFileTypes: true });
	const files: string[] = [];
	for (const entry of entries) {
		const full = path.join(dir, entry.name);
		if (entry.isDirectory()) {
			files.push(...(await findBareFiles(full)));
		} else if (entry.isFile() && entry.name.endsWith(".bare")) {
			files.push(full);
		}
	}
	return files;
}

const bareFiles = await findBareFiles(schemasDir);

await Promise.all(
	bareFiles.map(async (schemaPath) => {
		const rel = path.relative(schemasDir, schemaPath);
		const outputPath = path.join(distDir, rel.replace(/\.bare$/, ".ts"));
		await compileSchema({
			schemaPath,
			outputPath,
			config: { pedantic: false },
		});
		console.log(`Compiled ${rel}`);
	}),
);
