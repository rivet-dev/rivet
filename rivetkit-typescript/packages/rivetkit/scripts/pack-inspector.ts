import { existsSync } from "node:fs";
import { mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { create } from "tar";

const __dirname = dirname(fileURLToPath(import.meta.url));
const src = join(__dirname, "../../../../frontend/dist/inspector");
const destDir = join(__dirname, "../dist");
const destTar = join(destDir, "inspector.tar.gz");

if (!existsSync(src)) {
	throw new Error(
		`Inspector frontend not built yet. Run 'pnpm turbo build:inspector --filter=@rivetkit/engine-frontend' first.`,
	);
}

await mkdir(destDir, { recursive: true });
await create({ gzip: true, file: destTar, cwd: src }, ["."]);
console.log(`Packed inspector into ${destTar}`);
