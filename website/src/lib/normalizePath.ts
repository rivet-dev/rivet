/**
 * Normalizes a pathname by ensuring it has a trailing slash.
 * Root path "/" remains unchanged.
 *
 * @param path - The path to normalize
 * @returns The normalized path with trailing slash
 */
export function normalizePath(path: string): string {
	if (path === "/") return "/";
	return path.endsWith("/") ? path : `${path}/`;
}
