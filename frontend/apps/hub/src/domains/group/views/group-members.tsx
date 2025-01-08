import { groupMembersQueryOptions } from "@/domains/group/queries";
import { groupOnwerQueryOptions } from "@/domains/project/queries";
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
import { Icon, faCrown, faRightFromBracket } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { UserAvatar } from "../../user/components/user-avatar";

interface GroupMembersProps {
	groupId: string;
}

export function GroupMembers({ groupId }: GroupMembersProps) {
	const { data: groupOwnerIdentityId } = useSuspenseQuery(
		groupOnwerQueryOptions(groupId),
	);
	const { data } = useSuspenseQuery(groupMembersQueryOptions(groupId));

	return (
		<Card w="full">
			<CardHeader>
				<Flex items="center" gap="4" justify="between">
					<CardTitle>Members</CardTitle>
					<Flex gap="2">
						<Button asChild variant="secondary">
							<Link to="." search={{ modal: "invite" }}>
								Invite
							</Link>
						</Button>
						<Button asChild variant="secondary">
							<Link to="." search={{ modal: "leave" }}>
								<Icon icon={faRightFromBracket} />
							</Link>
						</Button>
					</Flex>
				</Flex>
			</CardHeader>
			<CardContent>
				<Grid gap="4">
					{data.members.map((member) => (
						<Flex
							key={member.identity.identityId}
							direction="row"
							gap="4"
							items="center"
						>
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
					))}
				</Grid>
			</CardContent>
		</Card>
	);
}
