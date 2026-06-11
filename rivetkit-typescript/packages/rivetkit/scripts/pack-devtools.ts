import { existsSync } from "node:fs";
import { copyFile, mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const src = join(__dirname, "../../devtools/dist/mod.js");
const destDir = join(__dirname, "../dist/devtools");
const dest = join(destDir, "mod.js");

if (!existsSync(src)) {
	throw new Error(
		`Devtools build not found at: ${src}. Run 'pnpm build -F @rivetkit/devtools' first.`,
	);
}

await mkdir(destDir, { recursive: true });
await copyFile(src, dest);
console.log(`Packed devtools into ${dest}`);
