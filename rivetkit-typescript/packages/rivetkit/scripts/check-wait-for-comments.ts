import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const WAIT_FOR_PATTERN = /\bvi\.waitFor\s*\(/;
const COMMENT_PATTERN = /^\s*\/\/\s+\S/;

async function collectFiles(dir: string): Promise<string[]> {
	const entries = await readdir(dir, { withFileTypes: true });
	const files = await Promise.all(
		entries.map(async (entry) => {
			const fullPath = path.join(dir, entry.name);
			if (entry.isDirectory()) {
				return collectFiles(fullPath);
			}
			if (entry.isFile() && /\.(ts|tsx|mts|cts)$/.test(entry.name)) {
				return [fullPath];
			}
			return [];
		}),
	);

	return files.flat();
}

async function main() {
	const scriptsDir = path.dirname(fileURLToPath(import.meta.url));
	const testsDir = path.resolve(scriptsDir, "../tests");
	const files = await collectFiles(testsDir);
	const violations: string[] = [];

	for (const filePath of files) {
		const source = await readFile(filePath, "utf8");
		const lines = source.split(/\r?\n/);

		for (const [index, line] of lines.entries()) {
			if (!WAIT_FOR_PATTERN.test(line)) continue;

			const previousLine = lines[index - 1] ?? "";
			if (!COMMENT_PATTERN.test(previousLine)) {
				violations.push(
					`${path.relative(testsDir, filePath)}:${index + 1} missing adjacent // comment for vi.waitFor`,
				);
			}
		}
	}

	if (violations.length === 0) return;

	console.error(
		"vi.waitFor calls must have an adjacent // comment explaining why polling is necessary:",
	);
	for (const violation of violations) {
		console.error(`- ${violation}`);
	}
	process.exit(1);
}

await main();
