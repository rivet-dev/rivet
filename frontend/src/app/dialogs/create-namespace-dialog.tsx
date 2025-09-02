import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import * as CreateNamespaceForm from "@/app/forms/create-namespace-form";
import { DialogFooter, DialogHeader, DialogTitle, Flex } from "@/components";
import { convertStringToId } from "@/lib/utils";
import {
	managerClient,
	namespacesQueryOptions,
} from "@/queries/manager-engine";

export default function CreateNamespacesDialogContent() {
	const queryClient = useQueryClient();
	const navigate = useNavigate();

	const { mutateAsync } = useMutation({
		mutationFn: async (data: { displayName: string; nameId: string }) => {
			const response = await managerClient.namespaces.create({
				displayName: data.displayName,
				name: data.nameId,
			});

			return response;
		},
		onSuccess: async (data) => {
			await queryClient.invalidateQueries(namespacesQueryOptions());
			navigate({
				to: "/ns/$namespace",
				params: { namespace: data.namespace.name },
			});
		},
	});

	return (
		<CreateNamespaceForm.Form
			onSubmit={async (values) => {
				await mutateAsync({
					displayName: values.name,
					nameId: values.slug || convertStringToId(values.name),
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
