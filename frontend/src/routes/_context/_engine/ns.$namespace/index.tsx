import { useSuspenseInfiniteQuery } from "@tanstack/react-query";
import { createFileRoute, Navigate, useSearch } from "@tanstack/react-router";
import { Actors } from "@/app/actors";
import { useDataProvider } from "@/components/actors";

export const Route = createFileRoute("/_context/_engine/ns/$namespace/")({
	component: RouteComponent,
});

export function RouteComponent() {
	const { actorId, n } = useSearch({ from: "/_context" });

	return (
		<>
			<Actors actorId={actorId} />
			{!n ? <BuildPrefiller /> : null}
		</>
	);
}

function BuildPrefiller() {
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
