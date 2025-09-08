import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useParams } from "@tanstack/react-router";
import * as CreateNamespaceForm from "@/app/forms/create-namespace-form";
import { DialogFooter, DialogHeader, DialogTitle, Flex } from "@/components";
import { useManager } from "@/components/actors";
import { convertStringToId } from "@/lib/utils";

const useCreateNamespace = () => {
	const queryClient = useQueryClient();
	const navigate = useNavigate();

	const params = useParams({ strict: false });

	const manager = useManager();

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

export default function CreateNamespacesDialogContent() {
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
			<DialogHeader>
				<DialogTitle>Create New Namespace</DialogTitle>
			</DialogHeader>
			<Flex gap="4" direction="col">
				<CreateNamespaceForm.Name />
				<CreateNamespaceForm.Slug />
			</Flex>
			<DialogFooter>
				<CreateNamespaceForm.Submit type="submit">
					Create
				</CreateNamespaceForm.Submit>
			</DialogFooter>
		</CreateNamespaceForm.Form>
	);
}
