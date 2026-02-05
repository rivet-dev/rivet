import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import posthog from "posthog-js";
import { useState } from "react";
import { Button, type DialogContentProps, Frame } from "@/components";
import { useCloudNamespaceDataProvider } from "@/components/actors";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { queryClient } from "@/queries/global";

interface ConfirmDeleteNamespaceContentProps extends DialogContentProps {
	displayName: string;
}

export default function ConfirmDeleteNamespaceContent({
	onClose,
	displayName,
}: ConfirmDeleteNamespaceContentProps) {
	const dataProvider = useCloudNamespaceDataProvider();
	const navigate = useNavigate();
	const [confirmValue, setConfirmValue] = useState("");

	const isConfirmed = confirmValue === displayName;

	const { mutate, isPending } = useMutation({
		...dataProvider.archiveCurrentNamespaceMutationOptions(),
		onSuccess: async () => {
			posthog.capture("namespace_deleted", {
				displayName,
			});
			queryClient.invalidateQueries();
			onClose?.();
			return navigate({
				to: "/orgs/$organization/projects/$project",
				from: "/orgs/$organization/projects/$project/ns/$namespace/settings",
			});
		},
	});

	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<div>Confirm archival of '{displayName}' namespace</div>
				</Frame.Title>
				<Frame.Description>
					Archiving this namespace will permanently remove all
					associated Rivet Actors, Runners, and configurations. This
					action cannot be undone.
				</Frame.Description>
			</Frame.Header>
			<Frame.Content>
				<div className="space-y-2">
					<Label htmlFor="confirm-delete-namespace">
						Type <span className="font-bold">{displayName}</span> to
						confirm
					</Label>
					<Input
						id="confirm-delete-namespace"
						value={confirmValue}
						onChange={(e) => setConfirmValue(e.target.value)}
						placeholder={displayName}
					/>
				</div>
			</Frame.Content>
			<Frame.Footer>
				<Button
					variant="destructive"
					isLoading={isPending}
					disabled={!isConfirmed}
					onClick={() => {
						mutate(undefined);
					}}
				>
					Archive
				</Button>
				<Button variant="secondary" onClick={onClose}>
					Cancel
				</Button>
			</Frame.Footer>
		</>
	);
}
