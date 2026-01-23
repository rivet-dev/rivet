import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const websiteDir = resolve(__dirname, "..");

const result = spawnSync("pnpm", ["astro", "build"], {
	cwd: websiteDir,
	stdio: "inherit",
});

if (result.status !== 0) {
	process.exit(result.status ?? 1);
}
