export const RECENT_PROJECTS_KEY = "rivet:recent_projects";
export const RECENT_NAMESPACES_KEY = "rivet:recent_namespaces";

type RecentlyVisitedType = typeof RECENT_PROJECTS_KEY | typeof RECENT_NAMESPACES_KEY;

type RecentMap = Record<string, number>;

function getRecentMap(key: RecentlyVisitedType): RecentMap {
	try {
		return JSON.parse(localStorage.getItem(key) ?? "{}");
	} catch {
		return {};
	}
}

export function recordRecentVisit(key: RecentlyVisitedType, name: string) {
	const map = getRecentMap(key);
	map[name] = Date.now();
	localStorage.setItem(key, JSON.stringify(map));
}

export function getRecentTimestamp(key: RecentlyVisitedType, name: string): number {
	return getRecentMap(key)[name] ?? 0;
}
