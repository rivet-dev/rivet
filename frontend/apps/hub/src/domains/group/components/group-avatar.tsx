import type { Rivet } from "@rivet-gg/api";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	type AvatarProps,
} from "@rivet-gg/components";

interface GroupAvatarProps
	extends Pick<Rivet.group.GroupSummary, "avatarUrl" | "displayName">,
		AvatarProps {}

export function GroupAvatar({
	avatarUrl,
	displayName,
	...props
}: GroupAvatarProps) {
	return (
		<Avatar {...props}>
			<AvatarImage src={avatarUrl} />
			<AvatarFallback>{displayName[0]}</AvatarFallback>
		</Avatar>
	);
}
