"use client";
import { Avatar, AvatarFallback, AvatarImage } from '@rivet-gg/components';

interface AuthorAvatarProps {
	name: string;
	avatarSrc: string;
	className?: string;
}

export function AuthorAvatar({ name, avatarSrc, className }: AuthorAvatarProps) {
	return (
		<Avatar className={className}>
			<AvatarFallback>{name[0]}</AvatarFallback>
			<AvatarImage src={avatarSrc} alt={name} />
		</Avatar>
	);
}
