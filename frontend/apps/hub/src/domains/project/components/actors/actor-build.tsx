import { useSuspenseQuery } from "@tanstack/react-query";
import { actorBuildQueryOptions } from "../../queries";
import { ActorObjectInspector } from "./console/actor-inspector";

interface ActorBuildProps {
	projectNameId: string;
	environmentNameId: string;
	buildId: string;
}

export function ActorBuild({
	projectNameId,
	environmentNameId,
	buildId,
}: ActorBuildProps) {
	const { data } = useSuspenseQuery(
		actorBuildQueryOptions({
			projectNameId,
			environmentNameId,
			buildId,
		}),
	);

	return <ActorObjectInspector data={{ build: data }} />;
}
