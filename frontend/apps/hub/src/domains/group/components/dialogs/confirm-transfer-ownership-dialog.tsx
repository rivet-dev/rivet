import { groupProjectsQueryOptions } from "@/domains/project/queries";
import { GuardEnterprise } from "@/lib/guards";
import {
	Button,
	Code,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	Link,
	Strong,
	Text,
} from "@rivet-gg/components";
import { useQuery, useSuspenseQuery } from "@tanstack/react-query";
import {
	groupMemberQueryOptions,
	useGroupTransferOwnershipMutation,
} from "../../queries";

interface ContentProps {
	groupId: string;
	identityId: string;
	onSuccess?: () => void;
}

export default function ConfirmTransferOwnershipDialogContent({
	groupId,
	identityId,
	onSuccess,
}: ContentProps) {
	const { data: group } = useSuspenseQuery(
		groupProjectsQueryOptions(groupId),
	);
	const { data: groupMember } = useQuery(
		groupMemberQueryOptions({ identityId, groupId }),
	);
	const { mutate, isPending } = useGroupTransferOwnershipMutation({
		onSuccess,
	});

	return (
		<>
			<DialogHeader>
				<DialogTitle>Confirm Ownership Transfer</DialogTitle>
				<DialogDescription asChild>
					<div>
						<Text>
							Are you sure you want to transfer ownership of group{" "}
							<Code>{group?.displayName}</Code>? This action{" "}
							<Strong>CANNOT</Strong> be undone.
						</Text>
						<GuardEnterprise>
							<Text>
								<Strong>
									As a developer group, transferring ownership
									will cause all billing related emails to be
									sent to the new owner. Your bank account
									information will stay attached to the group
									unless removed by a Rivet employee.
								</Strong>
							</Text>
						</GuardEnterprise>
						<Text>
							Contact{" "}
							<Link
								href="https://rivet.gg/support"
								target="_blank"
								rel="noreferrer"
							>
								Support
							</Link>{" "}
							for more info.
						</Text>
						<Text>
							New owner:{" "}
							<Strong>{groupMember?.identity.displayName}</Strong>
						</Text>
					</div>
				</DialogDescription>
			</DialogHeader>
			<DialogFooter>
				<Button
					type="submit"
					isLoading={isPending}
					onClick={() => {
						mutate({ groupId, newOwnerIdentityId: identityId });
					}}
				>
					Confirm
				</Button>
			</DialogFooter>
		</>
	);
}
