import { readFileSync } from "node:fs";
import type { LoadHook } from "node:module";
import { fileURLToPath } from "node:url";

export const load: LoadHook = async (url, context, nextLoad) => {
	if (url.endsWith(".sql")) {
		return {
			shortCircuit: true,
			format: "module",
			source: `export default ${JSON.stringify(readFileSync(fileURLToPath(url), "utf-8"))};`,
		};
	}
	return nextLoad(url, context);
};
