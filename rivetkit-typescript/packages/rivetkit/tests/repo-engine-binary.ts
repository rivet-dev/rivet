import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const REPO_ROOT = join(dirname(fileURLToPath(import.meta.url)), "../../../..");
const ENGINE_BINARY_NAME =
	process.platform === "win32" ? "rivet-engine.exe" : "rivet-engine";

export const REPO_ENGINE_BINARY = join(
	REPO_ROOT,
	"target",
	"debug",
	ENGINE_BINARY_NAME,
);
