import * as fs from "node:fs/promises";
import { resolve } from "node:path";
import { DocActorConfigSchema } from "../src/actor/config";
import { toJsonSchema } from "./schema-utils";

async function main() {
	const schema = toJsonSchema(DocActorConfigSchema);

	// Clean up the schema
	delete (schema as any).$schema;

	// Add metadata
	schema.title = "RivetKit Actor Configuration";
	schema.description = "Configuration schema for RivetKit actors. This is passed to the actor() function.";

	// Create output directory
	const outputDir = resolve(
		import.meta.dirname,
		"..",
		"..",
		"..",
		"artifacts",
	);
	await fs.mkdir(outputDir, { recursive: true });

	// Write the schema
	const outputPath = resolve(outputDir, "actor-config.json");
	await fs.writeFile(outputPath, JSON.stringify(schema, null, 2));
	console.log("Dumped actor config JSON schema to", outputPath);
}

main();
