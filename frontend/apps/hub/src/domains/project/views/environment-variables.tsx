import * as ProjectBackendEnvironmentVariablesForm from "@/domains/project/forms/backend-env-variables-form";
import {
	Card,
	CardContent,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { computeBackendEnvVariablesDiff } from "../helpers/backend-env-compute-diff-variables";
import {
	projectBackendEnvVariablesQueryOptions,
	useBackendUpdateVariablesMutation,
} from "../queries";

interface ProjectBackendEnvironmentVariablesProps {
	projectId: string;
	environmentId: string;
}

export function ProjectBackendEnvironmentVariables({
	environmentId,
	projectId,
}: ProjectBackendEnvironmentVariablesProps) {
	const { data } = useSuspenseQuery(
		projectBackendEnvVariablesQueryOptions({ projectId, environmentId }),
	);
	const { mutateAsync } = useBackendUpdateVariablesMutation();
	return (
		<ProjectBackendEnvironmentVariablesForm.Form
			onSubmit={async (values, form) => {
				const diff = computeBackendEnvVariablesDiff(
					data,
					values.variables,
				);
				if (diff.errors.length > 0) {
					for (const { idx, error } of diff.errors) {
						form.setError(`variables.${idx}.value`, {
							type: "manual",
							message: error,
						});
					}
					return;
				}
				return mutateAsync({
					projectId,
					environmentId,
					variables: diff.variables,
				});
			}}
			defaultValues={{
				variables: Object.entries(data).map(([key, value]) => ({
					key,
					value: value.text,
					isSecret: value.secret !== undefined,
				})),
			}}
		>
			<Card>
				<CardHeader>
					<CardTitle>Environment Variables</CardTitle>
				</CardHeader>
				<CardContent>
					<ProjectBackendEnvironmentVariablesForm.Variables />
				</CardContent>
				<CardFooter>
					<ProjectBackendEnvironmentVariablesForm.Submit>
						Save
					</ProjectBackendEnvironmentVariablesForm.Submit>
				</CardFooter>
			</Card>
		</ProjectBackendEnvironmentVariablesForm.Form>
	);
}
