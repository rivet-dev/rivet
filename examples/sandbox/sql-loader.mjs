import { readFile } from "node:fs/promises";

export async function resolve(specifier, context, nextResolve) {
	if (specifier.endsWith(".sql")) {
		return {
			shortCircuit: true,
			url: new URL(specifier, context.parentURL).href,
		};
	}

	return nextResolve(specifier, context);
}

export async function load(url, context, nextLoad) {
	if (url.endsWith(".sql")) {
		const source = await readFile(new URL(url), "utf8");
		return {
			format: "module",
			shortCircuit: true,
			source: `export default ${JSON.stringify(source)};`,
		};
	}

	return nextLoad(url, context);
}
