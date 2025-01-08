import { groupProjectsQueryOptions } from "@/domains/project/queries";
import {
	Button,
	Code,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	Text,
} from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { useGroupLeaveMutation } from "../../queries";

interface ConfirmLeaveGroupDialogContentProps {
	groupId: string;
	onSuccess?: () => void;
}

export default function ConfirmLeaveGroupDialogContent({
	groupId,
	onSuccess,
}: ConfirmLeaveGroupDialogContentProps) {
	const { data: group } = useSuspenseQuery(
		groupProjectsQueryOptions(groupId),
	);
	const { mutate, isPending } = useGroupLeaveMutation({
		onSuccess,
	});

	return (
		<>
			<DialogHeader>
				<DialogTitle>Confirm</DialogTitle>
				<DialogDescription asChild>
					<div>
						<Text>
							Are you sure you want to leave group
							<Code>{group?.displayName}</Code>?
						</Text>
					</div>
				</DialogDescription>
			</DialogHeader>
			<DialogFooter>
				<Button
					type="submit"
					isLoading={isPending}
					onClick={() => {
						mutate(groupId);
					}}
				>
					Confirm
				</Button>
			</DialogFooter>
		</>
	);
}
