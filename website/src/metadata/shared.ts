import path from "node:path";
import { fileURLToPath } from "node:url";

export const DOCS_BASE_URL = "https://rivet.gg/docs";
export const PROJECT_ROOT = fileURLToPath(new URL("../..", import.meta.url));

export function normalizeSlug(rawSlug: string) {
	let slug = rawSlug.replace(/\\/g, "/");
	if (slug === "index") return "";
	if (slug.endsWith("/index")) {
		slug = slug.slice(0, -"/index".length);
	}
	return slug;
}

export function resolveContentFile(filePath?: string) {
	if (!filePath) return null;
	return path.resolve(PROJECT_ROOT, filePath);
}
