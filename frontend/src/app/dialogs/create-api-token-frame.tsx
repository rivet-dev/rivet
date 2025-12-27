import { useMutation } from "@tanstack/react-query";
import { useRouteContext } from "@tanstack/react-router";
import { useState } from "react";
import * as CreateApiTokenForm from "@/app/forms/create-api-token-form";
import {
	Button,
	type DialogContentProps,
	DiscreteInput,
	Flex,
	Frame,
	Label,
} from "@/components";
import { queryClient } from "@/queries/global";

interface CreateApiTokenFrameContentProps extends DialogContentProps {}

/**
 * Convert duration string (e.g., "1y", "30d", "1h") to ISO 8601 timestamp
 */
function convertDurationToExpiresAt(duration: string): string | undefined {
	if (duration === "never") {
		return undefined;
	}

	const now = new Date();
	const match = duration.match(/^(\d+)([mhdy])$/);

	if (!match) {
		return undefined;
	}

	const value = Number.parseInt(match[1], 10);
	const unit = match[2];

	switch (unit) {
		case "m":
			now.setMinutes(now.getMinutes() + value);
			break;
		case "h":
			now.setHours(now.getHours() + value);
			break;
		case "d":
			now.setDate(now.getDate() + value);
			break;
		case "y":
			now.setFullYear(now.getFullYear() + value);
			break;
	}

	return now.toISOString();
}

export default function CreateApiTokenFrameContent({
	onClose,
}: CreateApiTokenFrameContentProps) {
	const { dataProvider } = useRouteContext({
		from: "/_context/_cloud/orgs/$organization/projects/$project",
	});
	const [createdToken, setCreatedToken] = useState<string | null>(null);

	const { mutateAsync, isPending } = useMutation(
		dataProvider.createApiTokenMutationOptions({
			onSuccess: async (data) => {
				setCreatedToken(data.apiToken);
				await queryClient.invalidateQueries(
					dataProvider.apiTokensQueryOptions(),
				);
			},
		}),
	);

	// Show the created token (only shown once!)
	if (createdToken) {
		return (
			<>
				<Frame.Header>
					<Frame.Title>Create Cloud API Token</Frame.Title>
				</Frame.Header>
				<Frame.Content>
					<div className="space-y-2">
						<Label>Your Cloud API Token</Label>
						<DiscreteInput value={createdToken} />
						<p className="text-sm text-destructive font-medium">
							This is the only time you'll see this token. Copy it
							now and store it securely.
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
		<CreateApiTokenForm.Form
			onSubmit={async (values) => {
				await mutateAsync({
					name: values.name,
					expiresAt: values.expiresIn
						? convertDurationToExpiresAt(values.expiresIn)
						: undefined,
				});
			}}
			defaultValues={{ name: "", expiresIn: "1y" }}
		>
			<Frame.Header>
				<Frame.Title>Create Cloud API Token</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<Flex gap="4" direction="col">
					<CreateApiTokenForm.Name />
					<CreateApiTokenForm.ExpiresIn />
				</Flex>
			</Frame.Content>
			<Frame.Footer>
				<CreateApiTokenForm.Submit type="submit" isLoading={isPending}>
					Create
				</CreateApiTokenForm.Submit>
				<Button variant="secondary" onClick={onClose}>
					Cancel
				</Button>
			</Frame.Footer>
		</CreateApiTokenForm.Form>
	);
}
