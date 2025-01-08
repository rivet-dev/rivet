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
	useGroupBanMemberMutation,
} from "../../queries";

interface ConfirmMemberBanDialogContentProps {
	groupId: string;
	identityId: string;
	onSuccess?: () => void;
}

export default function ConfirmMemberBanDialogContent({
	groupId,
	identityId,
	onSuccess,
}: ConfirmMemberBanDialogContentProps) {
	const { data: group } = useSuspenseQuery(
		groupProjectsQueryOptions(groupId),
	);
	const { data: groupMember } = useQuery(
		groupMemberQueryOptions({ identityId, groupId }),
	);
	const { mutate, isPending } = useGroupBanMemberMutation({
		onSuccess,
	});

	return (
		<>
			<DialogHeader>
				<DialogTitle>Confirm Member Ban</DialogTitle>
				<DialogDescription asChild>
					<div>
						<Text>
							Are you sure you want to ban{" "}
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
