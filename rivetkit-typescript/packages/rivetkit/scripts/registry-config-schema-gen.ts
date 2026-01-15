import * as fs from "node:fs/promises";
import { resolve } from "node:path";
import { DocRegistryConfigSchema } from "../src/registry/config/index";
import { toJsonSchema } from "./schema-utils";

async function main() {
	const schema = toJsonSchema(DocRegistryConfigSchema);

	// Clean up the schema
	delete (schema as any).$schema;

	// Add metadata
	schema.title = "RivetKit Registry Configuration";
	schema.description = "Configuration schema for RivetKit registry. This is typically passed to the setup() function.";

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
	const outputPath = resolve(outputDir, "registry-config.json");
	await fs.writeFile(outputPath, JSON.stringify(schema, null, 2));
	console.log("Dumped registry config JSON schema to", outputPath);
}

main();
