import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import posthog from "posthog-js";
import { useState } from "react";
import { Button, type DialogContentProps, Frame } from "@/components";
import { useCloudProjectDataProvider } from "@/components/actors";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { queryClient } from "@/queries/global";

interface ConfirmDeleteProjectContentProps extends DialogContentProps {
	displayName: string;
}

export default function ConfirmDeleteProjectContent({
	onClose,
	displayName,
}: ConfirmDeleteProjectContentProps) {
	const dataProvider = useCloudProjectDataProvider();
	const navigate = useNavigate();
	const [confirmValue, setConfirmValue] = useState("");

	const isConfirmed = confirmValue === displayName;

	const { mutate, isPending } = useMutation({
		...dataProvider.archiveCurrentProjectMutationOptions(),
		onSuccess: async () => {
			posthog.capture("project_deleted", {
				displayName,
			});
			queryClient.invalidateQueries();
			onClose?.();
			return navigate({
				to: "/orgs/$organization",
				from: "/orgs/$organization/projects/$project",
			});
		},
	});

	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<div>Confirm archival of '{displayName}' project</div>
				</Frame.Title>
				<Frame.Description>
					Archiving this project will permanently remove all associated
					namespaces, Rivet Actors, Runners, and configurations. This
					action cannot be undone.
				</Frame.Description>
			</Frame.Header>
			<Frame.Content>
				<div className="space-y-2">
					<Label htmlFor="confirm-delete-project">
						Type <span className="font-bold">{displayName}</span> to
						confirm
					</Label>
					<Input
						id="confirm-delete-project"
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
