import * as GroupInviteForm from "@/domains/group/forms/group-invite-form";
import {
	Button,
	CopyArea,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	Flex,
	Label,
	Text,
	timing,
} from "@rivet-gg/components";
import { useGroupInviteMutation } from "../../queries";

interface CreateGroupInviteDialogContentContentProps {
	groupId: string;
	onClose?: () => void;
}

export default function CreateGroupInviteDialogContent({
	groupId,
	onClose,
}: CreateGroupInviteDialogContentContentProps) {
	const { mutateAsync, data } = useGroupInviteMutation();

	if (data) {
		return (
			<>
				<DialogHeader>
					<DialogTitle>Create Group Invite</DialogTitle>
				</DialogHeader>
				<Flex direction="col" gap="4">
					<Text>
						Share this code or link to allow people to join your
						group.
					</Text>
					<Flex direction="col" gap="2">
						<Label>Code</Label>
						<CopyArea value={data.code} />
					</Flex>
					<Flex direction="col" gap="2">
						<Label>Link</Label>
						<CopyArea
							value={`${window.location.origin}/invite/${data.code}`}
						/>
					</Flex>
				</Flex>
				<DialogFooter onClick={onClose}>
					<Button variant="secondary">Close</Button>
				</DialogFooter>
			</>
		);
	}

	return (
		<>
			<GroupInviteForm.Form
				onSubmit={async (values) => {
					await mutateAsync({
						groupId,
						ttl: values.expTime === 0 ? undefined : values.expTime,
						useCount: values.isInfinite
							? undefined
							: values.usageCount,
					});
				}}
				defaultValues={{
					isInfinite: true,
					expTime: timing.minutes(30),
					usageCount: 10,
				}}
			>
				<DialogHeader>
					<DialogTitle>Create Group Invite</DialogTitle>
				</DialogHeader>
				<Flex gap="4" direction="col">
					<GroupInviteForm.ExpirationTime />
					<GroupInviteForm.Infinite />
					<GroupInviteForm.UsageCount />
				</Flex>
				<DialogFooter>
					<GroupInviteForm.Submit allowPristine type="submit">
						Create
					</GroupInviteForm.Submit>
				</DialogFooter>
			</GroupInviteForm.Form>
		</>
	);
}
