import { useMutation } from "@tanstack/react-query";
import * as CreateOrganizationForm from "@/app/forms/create-organization-form";
import { Button, type DialogContentProps, Frame } from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import { authClient } from "@/lib/auth";
import { queryClient } from "@/queries/global";

interface CreateOrganizationContentProps extends DialogContentProps {}

export default function CreateOrganizationContent({
	onClose,
}: CreateOrganizationContentProps) {
	const dataProvider = useCloudDataProvider();
	const { mutateAsync } = useMutation({
		mutationFn: async (values: { name: string }) => {
			// Slug is generated server-side; send a throwaway value to satisfy the API requirement.
			const result = await authClient.organization.create({
				name: values.name,
				slug: crypto.randomUUID(),
			});
			return result;
		},
		onSuccess: async () => {
			await queryClient.invalidateQueries(
				dataProvider.organizationsQueryOptions(),
			);
			onClose?.();
		},
	});

	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					Create Organization
				</Frame.Title>
				<Frame.Description>
					Fill out the details below to create a new organization.
				</Frame.Description>
			</Frame.Header>
			<CreateOrganizationForm.Form
				defaultValues={{ name: "" }}
				onSubmit={async (values) => {
					await mutateAsync(values);
				}}
			>
				<Frame.Content>
					<CreateOrganizationForm.Name />
				</Frame.Content>
				<Frame.Footer>
					<Button variant="secondary" onClick={onClose}>
						Close
					</Button>
					<CreateOrganizationForm.Submit type="submit">
						Create
					</CreateOrganizationForm.Submit>
				</Frame.Footer>
			</CreateOrganizationForm.Form>
		</>
	);
}
