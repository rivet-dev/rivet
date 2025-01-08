import {
	type MakeRouteMatchUnion,
	type StaticDataRouteOption,
	useMatches,
} from "@tanstack/react-router";

export function usePageLayout(): StaticDataRouteOption["layout"] {
	const matches = useMatches();
	return computePageLayout(matches);
}

export function computePageLayout(
	matches: MakeRouteMatchUnion[],
): StaticDataRouteOption["layout"] {
	let layout: StaticDataRouteOption["layout"] = "compact";

	for (const match of matches) {
		if (match.staticData.layout) {
			layout = match.staticData.layout;
		}
	}

	return layout;
}
