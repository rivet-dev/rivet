import type { Rivet } from "@rivet-gg/cloud";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import * as CreateProjectForm from "@/app/forms/create-project-form";
import { Flex, Frame } from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import { authClient } from "@/lib/auth";
import { features } from "@/lib/features";

const useDefaultOrg = () => {
	if (features.multitenancy) {
		// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
		const org = authClient.useActiveOrganization();
		return org.data?.id;
	}
	return undefined;
};

export default function CreateProjectFrameContent({
	organization,
	onSuccess,
	name,
}: {
	name?: string;
	organization?: string;
	// FIXME
	onSuccess?: (
		data: Rivet.ProjectsCreateResponse,
		vars: { displayName: string; organization: string },
	) => void;
}) {
	const queryClient = useQueryClient();
	const navigate = useNavigate();
	const provider = useCloudDataProvider();

	const defaultOrg = useDefaultOrg();

	const { mutateAsync } = useMutation({
		...provider.createProjectMutationOptions(),
		onSuccess: async (data, vars) => {
			await queryClient.refetchQueries(
				provider.currentOrgProjectsQueryOptions(),
			);

			return onSuccess
				? onSuccess(data, vars)
				: navigate({
						to: "/orgs/$organization/projects/$project",
						params: {
							organization: vars.organization,
							project: data.project.name,
						},
					});
		},
	});

	return (
		<CreateProjectForm.Form
			onSubmit={async (values) => {
				await mutateAsync({
					displayName: values.name,
					organization: values.organization,
				});
			}}
			defaultValues={{
				name: name,
				organization: organization ?? defaultOrg ?? "",
			}}
		>
			<Frame.Header>
				<Frame.Title>Create Project</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<Flex gap="4" direction="col">
					<CreateProjectForm.Organization />
					<CreateProjectForm.Name />
				</Flex>
			</Frame.Content>
			<Frame.Footer>
				<CreateProjectForm.Submit type="submit">
					Create
				</CreateProjectForm.Submit>
			</Frame.Footer>
		</CreateProjectForm.Form>
	);
}
