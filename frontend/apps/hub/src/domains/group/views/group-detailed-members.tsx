import { groupMembersQueryOptions } from "@/domains/group/queries";
import { groupOnwerQueryOptions } from "@/domains/project/queries";
import { useDialog } from "@/hooks/use-dialog";
import {
	Button,
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	Flex,
	Grid,
	Text,
} from "@rivet-gg/components";
import { Icon, faCrown } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { UserAvatar } from "../../user/components/user-avatar";
import { GroupMemberSettingsMenu } from "../components/group-member-settings-menu";

interface GroupDetailedMembersProps {
	groupId: string;
}

export function GroupDetailedMembers({ groupId }: GroupDetailedMembersProps) {
	const { data: groupOwnerIdentityId } = useSuspenseQuery(
		groupOnwerQueryOptions(groupId),
	);
	const { data } = useSuspenseQuery(groupMembersQueryOptions(groupId));

	const {
		open: confirmTransferOwnership,
		dialog: confirmTransferOwnershipDialog,
	} = useDialog.ConfirmTransferOwnership({ groupId });

	const { open: confirmMemberKick, dialog: confirmMemberKickDialog } =
		useDialog.ConfirmMemberKick({ groupId });

	const { open: confirmMemberBan, dialog: confirmMemberBanDialog } =
		useDialog.ConfirmMemberBan({ groupId });

	return (
		<Card w="full">
			<CardHeader>
				<Flex items="center" gap="4" justify="between">
					<CardTitle>Members</CardTitle>
					<Button variant="secondary" asChild>
						<Link to="." search={{ modal: "invite" }}>
							Invite
						</Link>
					</Button>
				</Flex>
			</CardHeader>
			<CardContent>
				{confirmTransferOwnershipDialog}
				{confirmMemberKickDialog}
				{confirmMemberBanDialog}
				<Grid gap="4">
					{data.members.map((member) => (
						<Flex
							key={member.identity.identityId}
							direction="row"
							gap="4"
							items="center"
						>
							<Flex w="full" gap="4">
								<UserAvatar {...member.identity} />
								<Flex gap="2" items="center">
									<Text>{member.identity.displayName}</Text>
									{groupOwnerIdentityId ===
										member.identity.identityId && (
										<Icon
											icon={faCrown}
											className="text-primary w-4"
										/>
									)}
								</Flex>
							</Flex>
							<GroupMemberSettingsMenu
								identityId={member.identity.identityId}
								groupId={groupId}
								onTransferOwnership={confirmTransferOwnership}
								onKick={confirmMemberKick}
								onBan={confirmMemberBan}
							/>
						</Flex>
					))}
				</Grid>
			</CardContent>
		</Card>
	);
}
