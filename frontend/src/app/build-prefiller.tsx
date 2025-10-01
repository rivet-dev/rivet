import { useSuspenseInfiniteQuery } from "@tanstack/react-query";
import { Navigate } from "@tanstack/react-router";
import { useDataProvider } from "@/components/actors";

export function BuildPrefiller() {
	const { data } = useSuspenseInfiniteQuery(
		useDataProvider().buildsQueryOptions(),
	);

	if (data[0]) {
		return (
			<Navigate
				to="."
				search={(search) => ({ ...search, n: [data[0].name] })}
			/>
		);
	}
	return null;
}
