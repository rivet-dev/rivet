import { useSuspenseQuery } from "@tanstack/react-query";
import {
	projectEnvironmentQueryOptions,
	projectVersionQueryOptions,
} from "../queries";
import { EnvironmentVersionTitle } from "./environment-version-title";

interface CurrentEnvironmentVersionTitleProps {
	environmentId: string;
	projectId: string;
}

export function CurrentEnvironmentVersionTitle({
	environmentId,
	projectId,
}: CurrentEnvironmentVersionTitleProps) {
	const {
		data: { namespace: environment },
	} = useSuspenseQuery(
		projectEnvironmentQueryOptions({ projectId, environmentId }),
	);

	const { data: version } = useSuspenseQuery(
		projectVersionQueryOptions({
			projectId,
			versionId: environment.versionId,
		}),
	);

	return (
		<EnvironmentVersionTitle
			environment={environment.displayName}
			version={version?.displayName ?? "Unknown"}
		/>
	);
}
