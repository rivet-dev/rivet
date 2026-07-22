import path from "node:path";
import { fileURLToPath } from "node:url";

export const SITE_BASE_URL = "https://rivet.dev";
export const DOCS_BASE_URL = `${SITE_BASE_URL}/docs`;
export const PROJECT_ROOT = fileURLToPath(new URL("../..", import.meta.url));

export function getDocsPath(slug: string) {
	if (slug === "integrations" || slug.startsWith("integrations/")) {
		return `/${slug}`;
	}

	return slug ? `/docs/${slug}` : "/docs";
}

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
