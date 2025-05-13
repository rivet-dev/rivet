import type { Rivet } from "@rivet-gg/api-full";
import { Avatar, AvatarFallback, AvatarImage } from "@rivet-gg/components";

interface UserAvatarProps
	extends Pick<Rivet.identity.Handle, "avatarUrl" | "displayName"> {}

export function UserAvatar({ avatarUrl, displayName }: UserAvatarProps) {
	return (
		<Avatar>
			<AvatarImage src={avatarUrl} />
			<AvatarFallback>{displayName[0]}</AvatarFallback>
		</Avatar>
	);
}
