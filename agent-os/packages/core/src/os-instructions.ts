import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const FIXTURE_PATH = resolve(__dirname, "../fixtures/AGENTOS_SYSTEM_PROMPT.md");

/**
 * Read the base OS instructions from the fixture file, optionally appending
 * additional instructions.
 */
export function getOsInstructions(additional?: string): string {
	const base = readFileSync(FIXTURE_PATH, "utf-8");
	if (additional) {
		return `${base}\n${additional}`;
	}
	return base;
}
