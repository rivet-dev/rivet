import fs from "node:fs";
import path from "node:path";
import { PROJECT_ROOT } from "./paths";

function packageJsonName(): string | undefined {
	try {
		const pkgPath = path.join(PROJECT_ROOT, "package.json");
		return (JSON.parse(fs.readFileSync(pkgPath, "utf8")) as { name?: string })
			.name;
	} catch {
		return undefined;
	}
}

let memo: string | undefined;

/** `SERVICE_NAME`, then `RENDER_SERVICE_NAME`, else `package.json` name. */
export function serviceName(): string {
	if (memo !== undefined) return memo;
	const fromEnv =
		process.env.SERVICE_NAME?.trim() ||
		process.env.RENDER_SERVICE_NAME?.trim();
	memo = fromEnv || (packageJsonName() ?? "service");
	return memo;
}
