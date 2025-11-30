import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { useRouteContext } from "@tanstack/react-router";
import {
	Button,
	type DialogContentProps,
	DiscreteInput,
	Flex,
	Frame,
	Label,
} from "@/components";
import { queryClient } from "@/queries/global";
import * as CreateApiKeyForm from "@/app/forms/create-api-key-form";

interface CreateApiKeyFrameContentProps extends DialogContentProps {}

export default function CreateApiKeyFrameContent({
	onClose,
}: CreateApiKeyFrameContentProps) {
	const { dataProvider } = useRouteContext({
		from: "/_context/_cloud/orgs/$organization/projects/$project",
	});
	const [createdKey, setCreatedKey] = useState<string | null>(null);

	const { mutateAsync, isPending } = useMutation(
		dataProvider.createApiKeyMutationOptions({
			onSuccess: async (data) => {
				setCreatedKey(data.apiKey);
				await queryClient.invalidateQueries(
					dataProvider.apiKeysQueryOptions(),
				);
			},
		}),
	);

	// Show the created key (only shown once!)
	if (createdKey) {
		return (
			<>
				<Frame.Header>
					<Frame.Title>API Key Created</Frame.Title>
					<Frame.Description>
						Copy this API key now. You won't be able to see it again!
					</Frame.Description>
				</Frame.Header>
				<Frame.Content>
					<div className="space-y-2">
						<Label>Your API Key</Label>
						<DiscreteInput value={createdKey} />
						<p className="text-sm text-destructive font-medium">
							This is the only time you'll see this key. Copy it now
							and store it securely.
						</p>
					</div>
				</Frame.Content>
				<Frame.Footer>
					<Button variant="default" onClick={onClose}>
						Done
					</Button>
				</Frame.Footer>
			</>
		);
	}

	return (
		<CreateApiKeyForm.Form
			onSubmit={async (values) => {
				await mutateAsync({
					name: values.name,
					expiresAt: values.expiresAt || undefined,
				});
			}}
			defaultValues={{ name: "", expiresAt: "" }}
		>
			<Frame.Header>
				<Frame.Title>Create API Key</Frame.Title>
				<Frame.Description>
					Create a new API key for programmatic access to this project.
				</Frame.Description>
			</Frame.Header>
			<Frame.Content>
				<Flex gap="4" direction="col">
					<CreateApiKeyForm.Name />
					<CreateApiKeyForm.ExpiresAt />
				</Flex>
			</Frame.Content>
			<Frame.Footer>
				<CreateApiKeyForm.Submit type="submit" isLoading={isPending}>
					Create
				</CreateApiKeyForm.Submit>
				<Button variant="secondary" onClick={onClose}>
					Cancel
				</Button>
			</Frame.Footer>
		</CreateApiKeyForm.Form>
	);
}
