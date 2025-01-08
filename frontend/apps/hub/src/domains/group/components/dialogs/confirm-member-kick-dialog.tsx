import { groupProjectsQueryOptions } from "@/domains/project/queries";
import {
	Button,
	Code,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	Strong,
	Text,
} from "@rivet-gg/components";
import { useQuery, useSuspenseQuery } from "@tanstack/react-query";
import {
	groupMemberQueryOptions,
	useGroupKickMemberMutation,
} from "../../queries";

interface ConfirmMemberKickDialogContentProps {
	groupId: string;
	identityId: string;
	onSuccess?: () => void;
}

export default function ConfirmMemberKickDialogContent({
	groupId,
	identityId,
	onSuccess,
}: ConfirmMemberKickDialogContentProps) {
	const { data: group } = useSuspenseQuery(
		groupProjectsQueryOptions(groupId),
	);
	const { data: groupMember } = useQuery(
		groupMemberQueryOptions({ identityId, groupId }),
	);
	const { mutate, isPending } = useGroupKickMemberMutation({
		onSuccess,
	});

	return (
		<>
			<DialogHeader>
				<DialogTitle>Confirm Member Kick</DialogTitle>
				<DialogDescription asChild>
					<div>
						<Text>
							Are you sure you want to kick{" "}
							<Strong>{groupMember?.identity.displayName}</Strong>{" "}
							from group <Code>{group?.displayName}</Code>?
						</Text>
					</div>
				</DialogDescription>
			</DialogHeader>
			<DialogFooter>
				<Button
					type="submit"
					isLoading={isPending}
					onClick={() => {
						mutate({ groupId, identityId });
					}}
				>
					Confirm
				</Button>
			</DialogFooter>
		</>
	);
}
