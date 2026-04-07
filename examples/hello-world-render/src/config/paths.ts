import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));

/** Repository root (directory that contains `package.json`). */
export const PROJECT_ROOT = path.join(here, "..", "..");
