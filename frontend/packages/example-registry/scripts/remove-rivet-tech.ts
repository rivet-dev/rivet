import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

async function main() {
	const examplesDir = path.join(__dirname, "../../../../examples");

	// Read all example directories
	const entries = await fs.readdir(examplesDir, { withFileTypes: true });
	const exampleDirs = entries.filter((entry) => entry.isDirectory());

	let updatedCount = 0;

	for (const dir of exampleDirs) {
		const packageJsonPath = path.join(examplesDir, dir.name, "package.json");

		try {
			// Read package.json
			const content = await fs.readFile(packageJsonPath, "utf-8");
			const packageJson = JSON.parse(content);

			// Skip if no template metadata
			if (!packageJson.template) {
				continue;
			}

			// Remove "rivet" from technologies
			const originalTechnologies = packageJson.template.technologies;
			const filteredTechnologies = originalTechnologies.filter(
				(tech: string) => tech !== "rivet",
			);

			// Only update if there was a change
			if (filteredTechnologies.length !== originalTechnologies.length) {
				packageJson.template.technologies = filteredTechnologies;

				// Write back with pretty formatting
				await fs.writeFile(
					packageJsonPath,
					JSON.stringify(packageJson, null, "\t") + "\n",
					"utf-8",
				);

				console.log(`✅ Updated ${dir.name}/package.json`);
				updatedCount++;
			}
		} catch (error) {
			console.warn(`⚠️  Could not update ${dir.name}:`, error);
		}
	}

	console.log(`\n✅ Removed "rivet" from ${updatedCount} package.json files`);
}

main().catch((error) => {
	console.error("Script failed:", error);
	process.exit(1);
});
