import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import * as CreateClusterForm from "@/app/forms/create-cluster-form";
import { Flex, Frame } from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import { authClient } from "@/lib/auth";
import { features } from "@/lib/features";

const useDefaultOrg = () => {
	if (features.platform) {
		// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
		const org = authClient.useActiveOrganization();
		return org.data?.id;
	}
	return undefined;
};

export default function CreateClusterFrameContent({
	organization,
}: {
	organization?: string;
}) {
	const navigate = useNavigate();
	const provider = useCloudDataProvider();

	const defaultOrg = useDefaultOrg();

	const { mutateAsync } = useMutation({
		...provider.createByocClusterMutationOptions(),
	});

	return (
		<CreateClusterForm.Form
			onSubmit={async (values) => {
				await mutateAsync({
					organization: values.organization,
					cluster: values.name,
				});

				await navigate({
					to: "/orgs/$organization/clusters/$cluster",
					params: {
						organization: values.organization,
						cluster: values.name,
					},
				});
			}}
			defaultValues={{
				name: "",
				organization: organization ?? defaultOrg ?? "",
			}}
		>
			<Frame.Header>
				<Frame.Title>Create new cluster</Frame.Title>
				<Frame.Description>
					Bring Your Own Cloud: the cluster runs on your own
					infrastructure.
				</Frame.Description>
			</Frame.Header>
			<Frame.Content>
				<Flex gap="4" direction="col">
					<CreateClusterForm.Organization />
					<CreateClusterForm.Name />
				</Flex>
			</Frame.Content>
			<Frame.Footer className="flex-row justify-end">
				<CreateClusterForm.DefaultSubmit />
			</Frame.Footer>
		</CreateClusterForm.Form>
	);
}
