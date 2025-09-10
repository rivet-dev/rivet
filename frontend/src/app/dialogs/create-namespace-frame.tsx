import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useParams } from "@tanstack/react-router";
import * as CreateNamespaceForm from "@/app/forms/create-namespace-form";
import { Flex, Frame } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { convertStringToId } from "@/lib/utils";

const useCreateNamespace = () => {
	const queryClient = useQueryClient();
	const navigate = useNavigate();

	const params = useParams({ strict: false });

	const manager = useEngineCompatDataProvider();

	return useMutation(
		manager.createNamespaceMutationOptions({
			onSuccess: async (data) => {
				// Invalidate all queries to ensure fresh data
				await queryClient.invalidateQueries(
					manager.namespacesQueryOptions(),
				);

				if (__APP_TYPE__ === "cloud") {
					if (!params.project || !params.organization) {
						throw new Error("Missing required parameters");
					}
					// Navigate to the newly created namespace
					navigate({
						to: "/orgs/$organization/projects/$project/ns/$namespace",
						params: {
							organization: params.organization,
							project: params.project,
							namespace: data.name,
						},
					});
					return;
				}

				navigate({
					to: "/ns/$namespace",
					params: { namespace: data.name },
				});
			},
		}),
	);
};

export default function CreateNamespacesFrameContent() {
	const { mutateAsync } = useCreateNamespace();

	return (
		<CreateNamespaceForm.Form
			onSubmit={async (values) => {
				await mutateAsync({
					displayName: values.name,
					name: values.slug || convertStringToId(values.name),
				});
			}}
			defaultValues={{ name: "", slug: "" }}
		>
			<Frame.Header>
				<Frame.Title>Create New Namespace</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<Flex gap="4" direction="col">
					<CreateNamespaceForm.Name />
					<CreateNamespaceForm.Slug />
				</Flex>
			</Frame.Content>
			<Frame.Footer>
				<CreateNamespaceForm.Submit type="submit">
					Create
				</CreateNamespaceForm.Submit>
			</Frame.Footer>
		</CreateNamespaceForm.Form>
	);
}
