import { groupOnwerQueryOptions } from "@/domains/project/queries";
import { selfProfileIdentityIdQueryOptions } from "@/domains/user/queries";
import {
	Button,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@rivet-gg/components";
import { Icon, faEllipsisVertical } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";

interface GroupMemberInfo {
	identityId: string;
	groupId: string;
}

interface GroupMemberSettingsMenuProps extends GroupMemberInfo {
	onTransferOwnership?: (data: GroupMemberInfo) => void;
	onKick?: (data: GroupMemberInfo) => void;
	onBan?: (data: GroupMemberInfo) => void;
}

export function GroupMemberSettingsMenu({
	groupId,
	identityId,
	onTransferOwnership,
	onKick,
	onBan,
}: GroupMemberSettingsMenuProps) {
	const { data: selfProfileIdentityId } = useSuspenseQuery(
		selfProfileIdentityIdQueryOptions(),
	);
	const { data: groupOwnerIdentityId } = useSuspenseQuery(
		groupOnwerQueryOptions(groupId),
	);
	if (
		selfProfileIdentityId !== groupOwnerIdentityId ||
		groupOwnerIdentityId === identityId
	) {
		return null;
	}
	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button size="icon" variant="outline">
					<Icon className="size-3.5" icon={faEllipsisVertical} />
					<span className="sr-only">More</span>
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent align="end">
				{groupOwnerIdentityId !== identityId ? (
					<DropdownMenuItem
						onSelect={() =>
							onTransferOwnership?.({ groupId, identityId })
						}
					>
						Transfer ownership
					</DropdownMenuItem>
				) : null}
				<DropdownMenuItem
					onSelect={() => {
						onKick?.({ groupId, identityId });
					}}
				>
					Kick
				</DropdownMenuItem>
				<DropdownMenuItem
					onSelect={() => {
						onBan?.({ groupId, identityId });
					}}
				>
					Ban
				</DropdownMenuItem>
			</DropdownMenuContent>
		</DropdownMenu>
	);
}
