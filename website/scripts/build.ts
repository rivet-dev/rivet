import { spawnSync } from "node:child_process";
import { cpSync, existsSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const websiteDir = resolve(__dirname, "..");
const repoRoot = resolve(websiteDir, "..");

const result = spawnSync("pnpm", ["astro", "build"], {
	cwd: websiteDir,
	stdio: "inherit",
});

if (result.status !== 0) {
	process.exit(result.status ?? 1);
}

const distSkillsDir = resolve(websiteDir, "dist", "metadata", "skills");
const targetSkillsDir = resolve(repoRoot, "skills");

if (!existsSync(distSkillsDir)) {
	console.error(`Expected skills directory not found: ${distSkillsDir}`);
	process.exit(1);
}

// The website build emits metadata skills into dist, so copy it to the repo root.
rmSync(targetSkillsDir, { recursive: true, force: true });
cpSync(distSkillsDir, targetSkillsDir, { recursive: true });
